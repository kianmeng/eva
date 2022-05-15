#![warn(clippy::all, clippy::pedantic)]
use {
    super::uri,
    crate::{bookmarks, BOOKMARKS, CONFIG},
    gemview::GemView,
    gtk::{glib::clone, prelude::*},
    std::{
        fs::File,
        io::{
            BufReader,
            Read,
        },
    },
    url::Url,
};

#[derive(Clone, Debug)]
/// Allows persistent access to the tab label, spinner indicator and close button
pub struct Label {
    handle: gtk::Box,
    label: gtk::Label,
    spinner: gtk::Spinner,
    close_button: gtk::Button,
}

impl Default for Label {
    fn default() -> Self {
        let handle = gtk::builders::BoxBuilder::new()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(3)
            .build();
        let label = gtk::Label::new(Some("about:blank"));
        let spinner = gtk::Spinner::new();
        spinner.set_visible(false);
        let close_button = gtk::builders::ButtonBuilder::new()
            .icon_name("window-close-symbolic")
            .has_frame(false)
            .build();
        handle.append(&label);
        handle.append(&spinner);
        handle.append(&close_button);
        Self {
            handle,
            label,
            spinner,
            close_button,
        }
    }
}

impl Label {
    pub fn handle(&self) -> gtk::Box {
        self.handle.clone()
    }

    pub fn label(&self) -> gtk::Label {
        self.label.clone()
    }

    pub fn close_button(&self) -> gtk::Button {
        self.close_button.clone()
    }
}

#[derive(Clone, Debug)]
pub struct BookmarkEditor {
    popover: gtk::Popover,
    label: gtk::Label,
    name: gtk::Entry,
    description: gtk::Entry,
    url: gtk::Entry,
    tags: gtk::Entry,
}

impl Default for BookmarkEditor {
    fn default() -> Self {
        let grid = gtk::builders::GridBuilder::new()
            .row_spacing(5)
            .column_spacing(5)
            .build();
        let popover = gtk::builders::PopoverBuilder::new().child(&grid).build();
        let label = gtk::builders::LabelBuilder::new()
            .use_markup(true)
            .halign(gtk::Align::Center)
            .label("<b>Create Bookmark</b>")
            .build();
        grid.attach(&label, 0, 0, 2, 1);
        let name_label = gtk::Label::new(Some("Name"));
        grid.attach(&name_label, 0, 1, 1, 1);
        let name = gtk::Entry::new();
        grid.attach(&name, 1, 1, 1, 1);
        let desc_label = gtk::Label::new(Some("Description"));
        grid.attach(&desc_label, 0, 2, 1, 1);
        let description = gtk::Entry::new();
        grid.attach(&description, 1, 2, 1, 1);
        let url_label = gtk::Label::new(Some("Url"));
        grid.attach(&url_label, 0, 3, 1, 1);
        let url = gtk::Entry::new();
        grid.attach(&url, 1, 3, 1, 1);
        let tag_label = gtk::Label::new(Some("Tags"));
        tag_label.set_valign(gtk::Align::Center);
        grid.attach(&tag_label, 0, 4, 1, 1);
        let tags = gtk::Entry::new();
        grid.attach(&tags, 1, 4, 1, 1);
        let cancel = gtk::builders::ButtonBuilder::new()
            .hexpand(false)
            .halign(gtk::Align::Start)
            .label("Cancel")
            .build();
        grid.attach(&cancel, 0, 5, 1, 1);
        let accept = gtk::builders::ButtonBuilder::new()
            .hexpand(false)
            .halign(gtk::Align::End)
            .label("Accept")
            .css_classes(vec![String::from("suggested-action")])
            .build();
        grid.attach(&accept, 1, 5, 1, 1);
        let pop = popover.clone();
        cancel.connect_clicked(move |_| pop.popdown());
        let editor = Self {
            popover,
            label,
            name,
            description,
            url,
            tags,
        };
        let ed = editor.clone();
        accept.connect_clicked(move |_| {
            let bm = ed.to_bookmark();
            let mut bmarks = BOOKMARKS.lock().unwrap();
            bmarks.update(&bm);
            if let Err(e) = bmarks.save() {
                eprintln!("Error: {}", e);
            }
            ed.popover.popdown();
        });
        editor
    }
}

impl BookmarkEditor {
    pub fn popover(&self) -> gtk::Popover {
        self.popover.clone()
    }

    /*pub fn name(&self) -> gtk::Entry {
        self.name.clone()
    }

    pub fn description(&self) -> gtk::Entry {
        self.description.clone()
    }

    pub fn url(&self) -> gtk::Entry {
        self.url.clone()
    }

    pub fn tags(&self) -> gtk::Entry {
        self.tags.clone()
    }*/

    pub fn to_bookmark(&self) -> bookmarks::Bookmark {
        bookmarks::BookmarkBuilder::new()
            .name(self.name.text().as_str())
            .description(match self.description.text().as_str() {
                "" => None,
                s => Some(s),
            })
            .url(self.url.text().as_str())
            .tags(
                self.tags
                    .text()
                    .to_string()
                    .split_whitespace()
                    .map(std::string::ToString::to_string)
                    .collect(),
            )
            .build()
    }
}

#[derive(Clone, Debug)]
/// A small popover for user input
pub struct Input {
    popover: gtk::Popover,
    label: gtk::Label,
    entry: gtk::Entry,
}

impl Default for Input {
    fn default() -> Self {
        let label = gtk::Label::new(None);
        let entry = gtk::Entry::new();
        let vbox = gtk::Box::new(gtk::Orientation::Vertical, 3);
        vbox.append(&label);
        vbox.append(&entry);
        let popover = gtk::Popover::builder()
            .autohide(true)
            .child(&vbox)
            .has_arrow(false)
            .position(gtk::PositionType::Bottom)
            .build();
        Self {
            popover,
            label,
            entry,
        }
    }
}

impl Input {
    pub fn show(&self) {
        self.popover.popup();
    }

    pub fn request(&self, meta: &str) {
        self.label.set_label(meta);
        self.show();
    }
}

#[derive(Clone, Debug)]
pub struct Tab {
    tab: gtk::Box,
    label: Label,
    bookmark_editor: BookmarkEditor,
    input: Input,
    upload: gtk::FileChooserDialog,
    back_button: gtk::Button,
    forward_button: gtk::Button,
    reload_button: gtk::Button,
    addr_bar: gtk::SearchEntry,
    bookmark_button: gtk::MenuButton,
    viewer: GemView,
}

impl Default for Tab {
    fn default() -> Self {
        let name: String = std::iter::repeat_with(fastrand::alphanumeric)
            .take(10)
            .collect();
        let tab = gtk::builders::BoxBuilder::new()
            .orientation(gtk::Orientation::Vertical)
            .name(&name)
            .build();
        let hbox = gtk::builders::BoxBuilder::new()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(3)
            .margin_start(3)
            .margin_end(3)
            .margin_top(3)
            .margin_bottom(3)
            .build();
        tab.append(&hbox);
        let button_box = gtk::builders::BoxBuilder::new()
            .orientation(gtk::Orientation::Horizontal)
            .homogeneous(true)
            .css_classes(vec![String::from("linked")])
            .margin_end(15)
            .build();
        hbox.append(&button_box);
        let image = gtk::builders::ImageBuilder::new()
            .icon_name("go-previous-symbolic")
            .build();
        let back_button = gtk::builders::ButtonBuilder::new()
            .child(&image)
            .tooltip_text("Go back")
            .action_name("win.go_previous")
            .sensitive(false)
            .build();
        button_box.append(&back_button);
        let image = gtk::builders::ImageBuilder::new()
            .icon_name("go-next-symbolic")
            .build();
        let forward_button = gtk::builders::ButtonBuilder::new()
            .child(&image)
            .tooltip_text("Go forward")
            .action_name("win.go_next")
            .sensitive(false)
            .build();
        button_box.append(&forward_button);
        let image = gtk::builders::ImageBuilder::new()
            .icon_name("view-refresh-symbolic")
            .build();
        let reload_button = gtk::builders::ButtonBuilder::new()
            .child(&image)
            .tooltip_text("Reload page")
            .action_name("win.reload")
            .sensitive(false)
            .build();
        button_box.append(&reload_button);
        let addr_bar = gtk::builders::SearchEntryBuilder::new()
            .placeholder_text("Search or enter an address")
            .hexpand(true)
            .build();
        hbox.append(&addr_bar);
        let input = Input::default();
        let upload = gtk::FileChooserDialog::builder()
            .use_header_bar(1)
            .destroy_with_parent(true)
            .modal(true)
            .title("Choose a file to upload")
            .action(gtk::FileChooserAction::Open)
            .create_folders(true)
            .build();
        upload.add_button("Accept", gtk::ResponseType::Accept);
        upload.add_button("Cancel", gtk::ResponseType::Cancel);
        let input_button = gtk::MenuButton::builder()
            .has_frame(false)
            .popover(&input.popover)
            .visible(false)
            .build();
        hbox.append(&input_button);
        let bookmark_button = gtk::builders::MenuButtonBuilder::new()
            .icon_name("bookmark-new-symbolic")
            .tooltip_text("Bookmark current page")
            .build();
        hbox.append(&bookmark_button);
        let scroller = gtk::builders::ScrolledWindowBuilder::new()
            .hexpand(true)
            .vexpand(true)
            .propagate_natural_width(true)
            .css_classes(vec!["gemview".to_string()])
            .build();
        let viewer = GemView::new();
        viewer.set_margin_start(25);
        viewer.set_margin_end(25);
        viewer.set_margin_top(25);
        viewer.set_margin_bottom(25);
        viewer.set_css_classes(&["gemview"]);
        scroller.set_child(Some(&viewer));
        tab.append(&scroller);
        let bookmark_editor = BookmarkEditor::default();
        bookmark_button.set_popover(Some(&bookmark_editor.popover));

        Self {
            tab,
            label: Label::default(),
            input,
            upload,
            bookmark_editor,
            back_button,
            forward_button,
            reload_button,
            addr_bar,
            bookmark_button,
            viewer,
        }
    }
}

impl Tab {
    pub fn init() -> Self {
        let tab = Self::default();
        tab.set_fonts();
        tab.update_bookmark_editor();
        tab.back_button.set_sensitive(false);
        tab.forward_button.set_sensitive(false);
        tab
    }

    pub fn connect_signals(&self) {
        self.addr_bar()
            .connect_activate(clone!(@strong self as tab => move |bar| {
                let mut uri = String::from(bar.text());
                uri = uri::uri(&mut uri);
                tab.viewer().visit(&uri);
            }));
        self.viewer()
            .connect_page_load_redirect(clone!(@strong self as tab => move |_, uri| {
                tab.addr_bar().set_text(&uri);
            }));
        self.viewer().connect_request_unsupported_scheme(
            clone!(@strong self as tab => move |_, uri| {
                if let Some((scheme, _)) = uri.split_once(':') {
                    match scheme {
                        "eva" => tab.request_eva_page(&uri),
                        _ => {
                            if let Err(e) = mime_open::open(&uri) {
                                eprintln!("Error opening {}: {}", uri, e);
                            }
                        }
                    }
                }
            }),
        );
        let upload = self.upload.clone();
        self.viewer().connect_request_upload(move |_viewer, _url| {
            upload.show();
        });
        self.upload.connect_response(clone!(@strong self.viewer as viewer => move |dlg,response| {
            if response == gtk::ResponseType::Accept {
                if let Some(file) = dlg.file() {
                    if let Some(path) = file.path() {
                        if let Ok(f) = File::open(path) {
                            let mut data: Vec<u8> = vec![];
                            let mut reader = BufReader::new(f);
                            if reader.read_to_end(&mut data).is_ok() {
                                if let Ok(url) = Url::parse(&viewer.uri()) {
                                    viewer.post_spartan(url, data);
                                }
                            }
                        }
                    }
                }
            }
            dlg.hide();
        }));
    }

    pub fn request_input(&self, meta: &str, url: String, visibility: bool) {
        let viewer = self.viewer.clone();
        let popover = self.input.popover.clone();
        self.input.entry.set_visibility(visibility);
        self.input.entry.connect_activate(move |entry| {
            let response = entry.text();
            if response.as_str() != "" {
                let mut url = url.to_string();
                url.push('?');
                let response = urlencoding::encode(response.as_str());
                url.push_str(&response);
                viewer.visit(&url);
                popover.popdown();
            }
        });
        self.input.request(meta);
    }

    pub fn tab(&self) -> gtk::Box {
        self.tab.clone()
    }

    pub fn label(&self) -> Label {
        self.label.clone()
    }

    pub fn bookmark_editor(&self) -> BookmarkEditor {
        self.bookmark_editor.clone()
    }

    /*pub fn input(&self) -> Input {
        self.input.clone()
    }*/

    pub fn back_button(&self) -> gtk::Button {
        self.back_button.clone()
    }

    pub fn forward_button(&self) -> gtk::Button {
        self.forward_button.clone()
    }

    pub fn reload_button(&self) -> gtk::Button {
        self.reload_button.clone()
    }

    pub fn addr_bar(&self) -> gtk::SearchEntry {
        self.addr_bar.clone()
    }

    /*pub fn bookmark_button(&self) -> gtk::MenuButton {
        self.bookmark_button.clone()
    }*/

    pub fn viewer(&self) -> GemView {
        self.viewer.clone()
    }

    pub fn set_fonts(&self) {
        let cfg = CONFIG.lock().unwrap().clone();
        self.viewer.set_font_paragraph(cfg.fonts.pg.to_pango());
        self.viewer.set_font_quote(cfg.fonts.quote.to_pango());
        self.viewer.set_font_pre(cfg.fonts.pre.to_pango());
        self.viewer.set_font_h1(cfg.fonts.h1.to_pango());
        self.viewer.set_font_h2(cfg.fonts.h2.to_pango());
        self.viewer.set_font_h3(cfg.fonts.h3.to_pango());
    }

    pub fn update_bookmark_editor(&self) {
        if let Ok(url) = Url::parse(self.viewer.uri().as_str()) {
            let bmarks = BOOKMARKS.lock().unwrap();
            let editor = &self.bookmark_editor;
            match bmarks.all.get(&self.viewer.uri()) {
                Some(b) => {
                    editor.label.set_label("<b>Edit Bookmark</b>");
                    editor.name.set_text(&b.name());
                    editor
                        .description
                        .set_text(&b.description().unwrap_or_default());
                    editor.url.set_text(&b.url());
                    editor.tags.set_text(&b.tags().join(" "));
                    self.bookmark_button
                        .set_icon_name("user-bookmarks-symbolic");
                }
                None => {
                    editor.label.set_label("<b>Create Bookmark</b>");
                    editor
                        .name
                        .set_text(url.host_str().unwrap_or("Unknown host"));
                    editor.description.set_text("");
                    editor.url.set_text(self.viewer.uri().as_str());
                    editor.tags.set_text("");
                    self.bookmark_button.set_icon_name("bookmark-new-symbolic");
                }
            }
        }
    }

    pub fn set_label(&self, label: &str, spin: bool) {
        self.label.label.set_label(label);
        if spin {
            self.label.spinner.show();
            self.label.spinner.start();
        } else {
            self.label.spinner.stop();
            self.label.spinner.hide();
        }
    }

    pub fn request_eva_page(&self, uri: &str) {
        if let Ok(url) = Url::parse(uri) {
            match url.host_str() {
                Some("bookmarks") => match url.path() {
                    "" | "/" => self.open_bookmarks(),
                    "/tags" | "/tags/" => self.open_bookmark_tags(),
                    p => {
                        let maybe_tag = p.replace("/tags/", "");
                        let bookmarks = BOOKMARKS.lock().unwrap();
                        if let Some(page) = bookmarks.tag_to_gmi(&maybe_tag) {
                            self.viewer.render_gmi(&page);
                            self.viewer.set_uri(uri);
                            self.addr_bar.set_text("uri");
                            self.set_label("bookmarks", false);
                        }
                    }
                },
                //Some("history") => {}
                Some("source") => {
                    self.view_source();
                }
                _ => {}
            }
        }
    }

    pub fn open_bookmarks(&self) {
        let bookmarks = BOOKMARKS.lock().unwrap();
        let page = bookmarks.to_gmi();
        self.viewer.render_gmi(&page);
        self.viewer.set_uri("eva://bookmarks");
        self.addr_bar.set_text("eva://bookmarks");
        self.bookmark_button.set_icon_name("bookmark-new-symbolic");
        self.set_label("bookmarks", false);
    }

    fn open_bookmark_tags(&self) {
        let bookmarks = BOOKMARKS.lock().unwrap();
        let page = bookmarks.tags_to_gmi();
        self.viewer.render_gmi(&page);
        self.viewer.set_uri("eva://bookmarks/tags");
        self.addr_bar.set_text("eva://bookmarks/tags");
        self.bookmark_button.set_icon_name("bookmark-new-symbolic");
        self.set_label("bookmarks", false);
    }

    pub fn view_source(&self) {
        let mime = self.viewer.buffer_mime();
        let content = self.viewer.buffer_content();
        if mime.starts_with("text") {
            let content = String::from_utf8_lossy(&content);
            self.viewer.render_text(&content);
            self.addr_bar.set_text("eva://source");
        }
    }
}
