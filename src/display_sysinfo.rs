use gtk::glib;
use gtk::prelude::{
    AdjustmentExt, BoxExt, ContainerExt, GridExt, LabelExt, ProgressBarExt, ScrolledWindowExt,
    ToggleButtonExt, WidgetExt,
};
use sysinfo::{self, ComponentExt, ProcessorExt, SystemExt};

use std::cell::RefCell;
use std::iter;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use crate::graph::Graph;
use crate::notebook::NoteBook;
use crate::settings::Settings;
use crate::utils::{connect_graph, format_number, RotateVec};

pub fn create_header(
    label_text: &str,
    parent_layout: &gtk::Box,
    display_graph: bool,
) -> gtk::CheckButton {
    let check_box = gtk::CheckButton::with_label("Graph view");
    check_box.set_active(display_graph);

    let label = gtk::Label::new(Some(label_text));
    let empty = gtk::Label::new(None);
    let grid = gtk::Grid::new();
    let horizontal_layout = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    horizontal_layout.pack_start(&gtk::Label::new(None), true, true, 0);
    horizontal_layout.pack_start(&check_box, false, false, 0);
    grid.attach(&empty, 0, 0, 3, 1);
    grid.attach_next_to(&label, Some(&empty), gtk::PositionType::Right, 3, 1);
    grid.attach_next_to(
        &horizontal_layout,
        Some(&label),
        gtk::PositionType::Right,
        3,
        1,
    );
    grid.set_column_homogeneous(true);
    parent_layout.pack_start(&grid, false, false, 15);
    check_box
}

pub fn create_progress_bar(
    non_graph_layout: &gtk::Grid,
    line: i32,
    label: &str,
    text: &str,
) -> gtk::ProgressBar {
    let p = gtk::ProgressBar::new();
    let l = gtk::Label::new(Some(label));

    p.set_text(Some(text));
    p.set_show_text(true);
    non_graph_layout.attach(&l, 0, line, 1, 1);
    non_graph_layout.attach(&p, 1, line, 11, 1);
    p
}

#[allow(dead_code)]
pub struct DisplaySysInfo {
    procs: Rc<RefCell<Vec<gtk::ProgressBar>>>,
    ram: gtk::ProgressBar,
    swap: gtk::ProgressBar,
    vertical_layout: gtk::Box,
    components: Vec<gtk::Label>,
    cpu_usage_history: Rc<RefCell<Graph>>,
    // 0 = RAM
    // 1 = SWAP
    ram_usage_history: Rc<RefCell<Graph>>,
    temperature_usage_history: Rc<RefCell<Graph>>,
    pub ram_check_box: gtk::CheckButton,
    pub swap_check_box: gtk::CheckButton,
    pub temperature_check_box: Option<gtk::CheckButton>,
}

impl DisplaySysInfo {
    pub fn new(
        sys: &Arc<Mutex<sysinfo::System>>,
        note: &mut NoteBook,
        settings: &Settings,
    ) -> DisplaySysInfo {
        let vertical_layout = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let mut procs = Vec::new();
        let scroll = gtk::ScrolledWindow::new(None::<&gtk::Adjustment>, None::<&gtk::Adjustment>);
        let mut components = vec![];

        // CPU
        let mut cpu_usage_history = Graph::new(None, false);
        cpu_usage_history.set_label_callbacks(Some(Box::new(|_| {
            [
                "100".to_string(),
                "50".to_string(),
                "0".to_string(),
                "%".to_string(),
            ]
        })));

        let sys = sys.lock().expect("failed to lock in DisplaySysInfo::new");
        // RAM
        let mut ram_usage_history = Graph::new(Some(sys.total_memory() as f64), true);
        ram_usage_history.set_label_callbacks(Some(Box::new(|v| {
            if v < 100_000. {
                [
                    v.to_string(),
                    format!("{}", v / 2.),
                    "0".to_string(),
                    "kB".to_string(),
                ]
            } else if v < 10_000_000. {
                [
                    format!("{:.1}", v / 1_000f64),
                    format!("{:.1}", v / 2_000f64),
                    "0".to_string(),
                    "MB".to_string(),
                ]
            } else if v < 10_000_000_000. {
                [
                    format!("{:.1}", v / 1_000_000f64),
                    format!("{:.1}", v / 2_000_000f64),
                    "0".to_string(),
                    "GB".to_string(),
                ]
            } else {
                [
                    format!("{:.1}", v / 1_000_000_000f64),
                    format!("{:.1}", v / 2_000_000_000f64),
                    "0".to_string(),
                    "TB".to_string(),
                ]
            }
        })));
        ram_usage_history.set_labels_width(70);

        // TEMPERATURE
        let mut temperature_usage_history = Graph::new(Some(1.), false);
        temperature_usage_history.set_overhead(Some(20.));
        temperature_usage_history.set_label_callbacks(Some(Box::new(|v| {
            [
                format!("{:.1}", v),
                format!("{:.1}", v / 2.),
                "0".to_string(),
                "°C".to_string(),
            ]
        })));
        temperature_usage_history.set_labels_width(70);

        let mut check_box3 = None;

        vertical_layout.set_spacing(5);
        vertical_layout.set_margin_top(10);
        vertical_layout.set_margin_bottom(10);

        let non_graph_layout = gtk::Grid::new();
        non_graph_layout.set_column_homogeneous(true);
        non_graph_layout.set_margin_end(5);
        let non_graph_layout2 = gtk::Grid::new();
        non_graph_layout2.set_column_homogeneous(true);
        non_graph_layout2.set_margin_start(5);
        let non_graph_layout3 = gtk::Box::new(gtk::Orientation::Vertical, 0);

        //
        // PROCESSOR PART
        //
        vertical_layout.pack_start(&gtk::Label::new(Some("Total CPU usage")), false, false, 7);
        procs.push(gtk::ProgressBar::new());
        {
            procs.push(gtk::ProgressBar::new());
            let p: &gtk::ProgressBar = &procs[0];

            p.set_margin_end(5);
            p.set_margin_start(5);
            p.set_show_text(true);
            let processor = sys.global_processor_info();
            p.set_text(Some(&format!("{:.1} %", processor.cpu_usage())));
            p.set_fraction(f64::from(processor.cpu_usage() / 100.));
            vertical_layout.add(p);
        }
        let check_box = create_header("Processors usage", &vertical_layout, settings.display_graph);
        for (i, pro) in sys.processors().iter().enumerate() {
            procs.push(gtk::ProgressBar::new());
            let p: &gtk::ProgressBar = &procs[i + 1];
            let l = gtk::Label::new(Some(&format!("{}", i)));

            p.set_text(Some(&format!("{:.1} %", pro.cpu_usage())));
            p.set_show_text(true);
            p.set_fraction(f64::from(pro.cpu_usage()));
            non_graph_layout.attach(&l, 0, i as i32 - 1, 1, 1);
            non_graph_layout.attach(p, 1, i as i32 - 1, 11, 1);
            cpu_usage_history.push(
                RotateVec::new(iter::repeat(0f64).take(61).collect()),
                &format!("processor {}", i),
                None,
            );
        }
        vertical_layout.add(&non_graph_layout);
        cpu_usage_history.attach_to(&vertical_layout);

        //
        // MEMORY PART
        //
        let check_box2 = create_header("Memory usage", &vertical_layout, settings.display_graph);
        let ram = create_progress_bar(&non_graph_layout2, 0, "RAM", "");
        let swap = create_progress_bar(&non_graph_layout2, 1, "Swap", "");
        vertical_layout.pack_start(&non_graph_layout2, false, false, 15);
        //vertical_layout.add(&non_graph_layout2);
        ram_usage_history.push(
            RotateVec::new(iter::repeat(0f64).take(61).collect()),
            "RAM",
            Some(4),
        );
        ram_usage_history.push(
            RotateVec::new(iter::repeat(0f64).take(61).collect()),
            "Swap",
            Some(2),
        );
        ram_usage_history.attach_to(&vertical_layout);

        //
        // TEMPERATURES PART
        //
        if !sys.components().is_empty() {
            check_box3 = Some(create_header(
                "Components' temperature",
                &vertical_layout,
                settings.display_graph,
            ));
            for component in sys.components() {
                let horizontal_layout = gtk::Box::new(gtk::Orientation::Horizontal, 10);
                // TODO: add max and critical temperatures as well
                let temp = gtk::Label::new(Some(&format!("{:.1} °C", component.temperature())));
                horizontal_layout.pack_start(
                    &gtk::Label::new(Some(component.label())),
                    true,
                    false,
                    0,
                );
                horizontal_layout.pack_start(&temp, true, false, 0);
                horizontal_layout.set_homogeneous(true);
                non_graph_layout3.add(&horizontal_layout);
                components.push(temp);
                temperature_usage_history.push(
                    RotateVec::new(iter::repeat(0f64).take(61).collect()),
                    component.label(),
                    None,
                );
            }
            vertical_layout.add(&non_graph_layout3);
            temperature_usage_history.attach_to(&vertical_layout);
        }

        //
        // Putting everyting into places now.
        //
        let cpu_usage_history = connect_graph(cpu_usage_history);
        let ram_usage_history = connect_graph(ram_usage_history);
        let temperature_usage_history = connect_graph(temperature_usage_history);

        scroll.add(&vertical_layout);
        note.create_tab("System usage", &scroll);

        // It greatly improves the scrolling on the system information tab. No more clipping.
        let adjustment = scroll.vadjustment();
        adjustment.connect_value_changed(
            glib::clone!(@weak cpu_usage_history, @weak ram_usage_history, @weak temperature_usage_history => move |_| {
            cpu_usage_history.borrow().invalidate();
            ram_usage_history.borrow().invalidate();
            temperature_usage_history.borrow().invalidate();
        }));

        let mut tmp = DisplaySysInfo {
            procs: Rc::new(RefCell::new(procs)),
            ram,
            swap,
            vertical_layout,
            components,
            cpu_usage_history: Rc::clone(&cpu_usage_history),
            ram_usage_history: Rc::clone(&ram_usage_history),
            ram_check_box: check_box.clone(),
            swap_check_box: check_box2.clone(),
            temperature_usage_history: Rc::clone(&temperature_usage_history),
            temperature_check_box: check_box3.clone(),
        };
        tmp.update_system_info(&sys, settings.display_fahrenheit);

        check_box.connect_toggled(
            glib::clone!(@weak non_graph_layout, @weak cpu_usage_history => move |c| {
                show_if_necessary(c, &cpu_usage_history.borrow(), &non_graph_layout);
            }),
        );
        check_box2.connect_toggled(
            glib::clone!(@weak non_graph_layout2, @weak ram_usage_history => move |c| {
                show_if_necessary(c, &ram_usage_history.borrow(), &non_graph_layout2);
            }),
        );
        if let Some(ref check_box3) = check_box3 {
            check_box3.connect_toggled(
                glib::clone!(@weak non_graph_layout3, @weak temperature_usage_history => move |c| {
                    show_if_necessary(c, &temperature_usage_history.borrow(), &non_graph_layout3);
                }),
            );
        }

        scroll.connect_show(
            glib::clone!(@weak cpu_usage_history, @weak ram_usage_history => move |_| {
                show_if_necessary(&check_box,
                                  &cpu_usage_history.borrow(), &non_graph_layout);
                show_if_necessary(&check_box2,
                                  &ram_usage_history.borrow(), &non_graph_layout2);
                if let Some(ref check_box3) = check_box3 {
                    show_if_necessary(check_box3,
                                      &temperature_usage_history.borrow(), &non_graph_layout3);
                }
            }),
        );
        tmp
    }

    pub fn set_size_request(&self, width: i32, height: i32) {
        self.cpu_usage_history
            .borrow()
            .area
            .set_size_request(width, height);
        self.ram_usage_history
            .borrow()
            .area
            .set_size_request(width, height);
        self.temperature_usage_history
            .borrow()
            .area
            .set_size_request(width, height);
    }

    pub fn set_checkboxes_state(&self, active: bool) {
        self.ram_check_box.set_active(active);
        self.swap_check_box.set_active(active);
        if let Some(ref temperature_check_box) = self.temperature_check_box {
            temperature_check_box.set_active(active);
        }
    }

    pub fn update_system_info(&mut self, sys: &sysinfo::System, display_fahrenheit: bool) {
        let disp = |total, used| {
            format!(
                "{} / {}",
                format_number(used * 1_000),
                format_number(total * 1_000) // We need to multiply to get the "right" unit.
            )
        };

        let total_ram = sys.total_memory();
        let used = sys.used_memory();
        self.ram.set_text(Some(&disp(total_ram, used)));
        if total_ram != 0 {
            self.ram.set_fraction(used as f64 / total_ram as f64);
        } else {
            self.ram.set_fraction(0.0);
        }
        {
            let mut r = self.ram_usage_history.borrow_mut();
            r.data[0].move_start();
            if let Some(p) = r.data[0].get_mut(0) {
                *p = used as f64;
            }
        }

        let total = ::std::cmp::max(sys.total_swap(), total_ram);
        let used = sys.used_swap();
        self.swap.set_text(Some(&disp(sys.total_swap(), used)));

        let mut fraction = if total != 0 {
            used as f64 / total as f64
        } else {
            0f64
        };
        if fraction.is_nan() {
            fraction = 0f64;
        }
        self.swap.set_fraction(fraction);
        {
            let mut r = self.ram_usage_history.borrow_mut();
            r.data[1].move_start();
            if let Some(p) = r.data[1].get_mut(0) {
                *p = used as f64;
            }
        }

        // temperature part
        let mut t = self.temperature_usage_history.borrow_mut();
        for (pos, (component, label)) in sys
            .components()
            .iter()
            .zip(self.components.iter())
            .enumerate()
        {
            t.data[pos].move_start();
            if let Some(t) = t.data[pos].get_mut(0) {
                *t = f64::from(component.temperature());
            }
            if let Some(t) = t.data[pos].get_mut(0) {
                *t = f64::from(component.temperature());
            }
            if display_fahrenheit {
                label.set_text(&format!("{:.1} °F", component.temperature() * 1.8 + 32.));
            } else {
                label.set_text(&format!("{:.1} °C", component.temperature()));
            }
        }
    }

    pub fn update_system_info_display(&mut self, sys: &sysinfo::System) {
        let v = &*self.procs.borrow_mut();
        let h = &mut *self.cpu_usage_history.borrow_mut();

        v[0].set_text(Some(&format!(
            "{:.1} %",
            sys.global_processor_info().cpu_usage()
        )));
        v[0].set_show_text(true);
        v[0].set_fraction(f64::from(sys.global_processor_info().cpu_usage() / 100.));
        for (i, pro) in sys.processors().iter().enumerate() {
            let i = i + 1;
            v[i].set_text(Some(&format!("{:.1} %", pro.cpu_usage())));
            v[i].set_show_text(true);
            v[i].set_fraction(f64::from(pro.cpu_usage() / 100.));
            h.data[i - 1].move_start();
            if let Some(h) = h.data[i - 1].get_mut(0) {
                *h = f64::from(pro.cpu_usage() / 100.);
            }
        }
        h.invalidate();
        self.ram_usage_history.borrow().invalidate();
        self.temperature_usage_history.borrow().invalidate();
    }
}

pub fn show_if_necessary<U: gtk::glib::IsA<gtk::ToggleButton>, T: WidgetExt>(
    check_box: &U,
    proc_horizontal_layout: &Graph,
    non_graph_layout: &T,
) {
    if check_box.is_active() {
        proc_horizontal_layout.show_all();
        non_graph_layout.hide();
    } else {
        non_graph_layout.show_all();
        proc_horizontal_layout.hide();
    }
}
