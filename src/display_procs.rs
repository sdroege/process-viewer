use gtk::glib::object::Cast;
use gtk::glib::Type;
use gtk::prelude::{
    BoxExt, ButtonExt, CellRendererExt, ContainerExt, EntryExt, GridExt, GtkListStoreExtManual,
    GtkWindowExt, OverlayExt, SearchBarExt, TreeModelExt, TreeModelFilterExt, TreeSelectionExt,
    TreeViewColumnExt, TreeViewExt, WidgetExt,
};
use gtk::{self, glib};

use sysinfo::{Pid, PidExt, Process, ProcessExt};

use crate::notebook::NoteBook;
use crate::utils::{create_button_with_image, format_number};

use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;

#[allow(dead_code)]
pub struct Procs {
    pub left_tree: gtk::TreeView,
    pub scroll: gtk::ScrolledWindow,
    pub current_pid: Rc<Cell<Option<Pid>>>,
    pub kill_button: gtk::Button,
    pub info_button: gtk::Button,
    pub vertical_layout: gtk::Box,
    pub list_store: gtk::ListStore,
    pub columns: Vec<gtk::TreeViewColumn>,
    pub filter_entry: gtk::Entry,
    pub search_bar: gtk::SearchBar,
    pub filter_button: gtk::Button,
}

impl Procs {
    pub fn new(
        proc_list: &HashMap<Pid, Process>,
        note: &mut NoteBook,
        window: &gtk::ApplicationWindow,
    ) -> Procs {
        let left_tree = gtk::TreeView::new();
        let scroll = gtk::ScrolledWindow::new(None::<&gtk::Adjustment>, None::<&gtk::Adjustment>);
        let current_pid = Rc::new(Cell::new(None));
        let kill_button = gtk::Button::with_label("End task");
        let info_button = gtk::Button::with_label("More information");

        let filter_button =
            create_button_with_image(include_bytes!("../assets/magnifier.png"), "Filter");

        // TODO: maybe add an 'X' button to close search as well?
        let overlay = gtk::Overlay::new();
        let filter_entry = gtk::Entry::new();
        let search_bar = gtk::SearchBar::new();

        // We put the filter entry at the right bottom.
        filter_entry.set_halign(gtk::Align::End);
        filter_entry.set_valign(gtk::Align::End);
        filter_entry.hide(); // By default, we don't show it.
        search_bar.connect_entry(&filter_entry);
        search_bar.set_show_close_button(true);

        overlay.add_overlay(&filter_entry);

        let mut columns: Vec<gtk::TreeViewColumn> = Vec::new();

        let list_store = gtk::ListStore::new(&[
            // The first four columns of the model are going to be visible in the view.
            Type::U32,    // pid
            Type::STRING, // name
            Type::STRING, // CPU
            Type::STRING, // mem
            Type::STRING, // disk I/O
            // These two will serve as keys when sorting by process name and CPU usage.
            Type::STRING, // name_lowercase
            Type::F32,    // CPU_f32
            Type::U64,    // mem
            Type::U64,    // disk I/O
        ]);

        for pro in proc_list.values() {
            if let Some(exe) = pro
                .exe()
                .file_name()
                .and_then(|f| f.to_str())
                .or_else(|| Some(pro.name()))
            {
                create_and_fill_model(
                    &list_store,
                    pro.pid().as_u32(),
                    pro.cmd(),
                    exe,
                    pro.cpu_usage(),
                    pro.memory() * 1_000,
                );
            }
        }

        left_tree.set_headers_visible(true);
        scroll.add(&left_tree);
        overlay.add(&scroll);
        let vertical_layout = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let horizontal_layout = gtk::Grid::new();

        left_tree.connect_cursor_changed(
            glib::clone!(@weak current_pid, @weak kill_button, @weak info_button => move |tree_view| {
                let selection = tree_view.selection();
                let (pid, ret) = if let Some((model, iter)) = selection.selected() {
                    if let Ok(x) = model.value(&iter, 0).get::<u32>() {
                        (Some(Pid::from_u32(x)), true)
                    } else {
                        (None, false)
                    }
                } else {
                    (None, false)
                };
                current_pid.set(pid);
                kill_button.set_sensitive(ret);
                info_button.set_sensitive(ret);
            }),
        );
        kill_button.set_sensitive(false);
        info_button.set_sensitive(false);

        vertical_layout.pack_start(&overlay, true, true, 0);
        horizontal_layout.attach(&info_button, 0, 0, 4, 1);
        horizontal_layout.attach_next_to(
            &kill_button,
            Some(&info_button),
            gtk::PositionType::Right,
            4,
            1,
        );
        horizontal_layout.attach_next_to(
            &filter_button,
            Some(&kill_button),
            gtk::PositionType::Right,
            1,
            1,
        );
        horizontal_layout.set_column_homogeneous(true);
        vertical_layout.pack_start(&horizontal_layout, false, true, 0);

        // The filter part.
        let filter_model = gtk::TreeModelFilter::new(&list_store, None);
        filter_model.set_visible_func(
            glib::clone!(@weak filter_entry => @default-return false, move |model, iter| {
                if !WidgetExt::is_visible(&filter_entry) || filter_entry.text_length() < 1 {
                    return true;
                }
                let text = filter_entry.text();
                    if text.is_empty() {
                        return true;
                    }
                    let text: &str = text.as_ref();
                    // TODO: Maybe add an option to make searches case sensitive?
                    let pid = model.value(iter, 0)
                                   .get::<u32>()
                                   .map(|p| p.to_string())
                                   .ok()
                                   .unwrap_or_else(String::new);
                    let name = model.value(iter, 1)
                                    .get::<String>()
                                    .map(|s| s.to_lowercase())
                                    .ok()
                                    .unwrap_or_else(String::new);
                    pid.contains(text) ||
                    text.contains(&pid) ||
                    name.contains(text) ||
                    text.contains(&name)
            }),
        );
        // For the filtering to be taken into account, we need to add it directly into the
        // "global" model.
        let sort_model = gtk::TreeModelSort::new(&filter_model);
        left_tree.set_model(Some(&sort_model));

        append_column("pid", &mut columns, &left_tree, None);
        append_column("process name", &mut columns, &left_tree, Some(200));
        append_column("cpu usage", &mut columns, &left_tree, None);
        append_column("memory usage", &mut columns, &left_tree, None);
        #[cfg(not(windows))]
        {
            append_column("disk I/O usage", &mut columns, &left_tree, None);
        }
        #[cfg(windows)]
        {
            append_column("I/O usage", &mut columns, &left_tree, None);
        }

        // When we click the "name" column the order is defined by the
        // "name_lowercase" effectively making the built-in comparator ignore case.
        columns[1].set_sort_column_id(5);
        // Likewise clicking the "CPU" column sorts by the "CPU_f32" one because
        // we want the order to be numerical not lexicographical.
        columns[2].set_sort_column_id(6);
        // The memory usage display has been improved, so to make efficient sort,
        // we have to separate the display and the actual number.
        columns[3].set_sort_column_id(7);
        // The disk I/O usage display has been improved, so to make efficient sort,
        // we have to separate the display and the actual number.
        columns[4].set_sort_column_id(8);

        filter_entry.connect_text_length_notify(move |_| {
            filter_model.refilter();
        });

        note.create_tab("Process list", &vertical_layout);

        filter_button.connect_clicked(glib::clone!(@weak filter_entry, @weak window => move |_| {
            if WidgetExt::is_visible(&filter_entry) {
                filter_entry.hide();
            } else {
                filter_entry.show_all();
                window.set_focus(Some(&filter_entry));
            }
        }));

        Procs {
            left_tree,
            scroll,
            current_pid,
            kill_button,
            info_button,
            vertical_layout: vertical_layout
                .downcast::<gtk::Box>()
                .expect("downcast failed"),
            list_store,
            columns,
            filter_entry,
            search_bar,
            filter_button,
        }
    }

    pub fn hide_filter(&self) {
        self.filter_entry.hide();
        self.filter_entry.set_text("");
        self.search_bar.set_search_mode(false);
    }
}

fn append_column(
    title: &str,
    v: &mut Vec<gtk::TreeViewColumn>,
    left_tree: &gtk::TreeView,
    max_width: Option<i32>,
) {
    let id = v.len() as i32;
    let renderer = gtk::CellRendererText::new();

    if title != "process name" {
        renderer.set_xalign(1.0);
    }

    let column = gtk::TreeViewColumn::new();
    column.set_title(title);
    column.set_resizable(true);
    if let Some(max_width) = max_width {
        column.set_max_width(max_width);
        column.set_expand(true);
    }
    column.set_min_width(10);
    column.pack_start(&renderer, true);
    column.add_attribute(&renderer, "text", id);
    column.set_clickable(true);
    column.set_sort_column_id(id);
    left_tree.append_column(&column);
    v.push(column);
}

pub fn create_and_fill_model(
    list_store: &gtk::ListStore,
    pid: u32,
    cmdline: &[String],
    name: &str,
    cpu: f32,
    memory: u64,
) {
    if cmdline.is_empty() || name.is_empty() {
        return;
    }
    list_store.insert_with_values(
        None,
        &[
            (0, &pid),
            (1, &name),
            (2, &format!("{:.1}", cpu)),
            (3, &format_number(memory)),
            (4, &String::new()),
            (5, &name.to_lowercase()),
            (6, &cpu),
            (7, &memory),
            (8, &0),
        ],
    );
}
