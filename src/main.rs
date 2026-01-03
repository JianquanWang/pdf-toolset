use gtk::{gdk, prelude::*};
use gtk::glib::Type as GlibType;
use gtk::{Application, ApplicationWindow, Box as GtkBox, Button, CheckButton, Entry, Frame, Label, Orientation, ProgressBar, ToggleButton};
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
mod backend;

fn main() {
    let app = Application::builder()
        .application_id("com.github.gemini.pdf-tools")
        .build();
    app.connect_activate(build_ui);
    app.run();
}

fn build_ui(app: &Application) {
    let files: Rc<RefCell<Vec<PathBuf>>> = Rc::new(RefCell::new(Vec::new()));

    let css = r#"
    .root { background-color: #f5f7fa; }
    .card { background-color: #ffffff; border-radius: 8px; padding: 10px; box-shadow: 0 2px 6px rgba(0,0,0,0.06); }
    .func-button { font-weight: 600; padding: 10px 14px; border-radius: 6px; }
    .preview-image { border-radius: 6px; border: 1px solid rgba(0,0,0,0.08); }
    "#;
    let provider = gtk::CssProvider::new();
    let _ = provider.load_from_data(css);
    if let Some(display) = gdk::Display::default() {
        gtk::style_context_add_provider_for_display(&display, &provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);
    }

    let vbox = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(12)
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(12)
        .margin_end(12)
        .build();
    vbox.style_context().add_class("root");

    // Drop area (Frame with label inside) to improve DnD handling
    let drop_frame = Frame::builder().label("Drop files here").build();
    drop_frame.set_size_request(400, 120);
    let drop_label = Label::builder().label("Drag and drop a PDF file here").build();
    drop_frame.set_child(Some(&drop_label));
    drop_frame.style_context().add_class("card");
    vbox.append(&drop_frame);

    let file_list_label = Label::builder().label("").build();
    vbox.append(&file_list_label);
    file_list_label.set_visible(false);

    // File pool box (shows checkboxes for each added file) inside a scroller
    let file_pool_box = GtkBox::builder().orientation(Orientation::Vertical).spacing(4).build();
    let file_pool_scroller = gtk::ScrolledWindow::builder().vexpand(true).min_content_height(120).build();
    file_pool_scroller.set_child(Some(&file_pool_box));
    vbox.append(&file_pool_scroller);

    let pool_controls = GtkBox::builder().orientation(Orientation::Horizontal).spacing(6).build();
    let select_all_btn = Button::with_label("Select All");
    let remove_selected_btn = Button::with_label("Remove Selected");
    pool_controls.append(&select_all_btn);
    pool_controls.append(&remove_selected_btn);
    vbox.append(&pool_controls);

    let add_files_btn = Button::with_label("Add files");
    vbox.append(&add_files_btn);

    let transform_frame = Frame::builder().label("Transform").build();
    let transform_box = GtkBox::builder().orientation(Orientation::Horizontal).spacing(6).margin_top(6).margin_bottom(6).build();
    transform_frame.set_child(Some(&transform_box));
    transform_frame.style_context().add_class("card");

    let export_frame = Frame::builder().label("Export / Convert").build();
    let export_box = GtkBox::builder().orientation(Orientation::Horizontal).spacing(6).margin_top(6).margin_bottom(6).build();
    export_frame.set_child(Some(&export_box));
    export_frame.style_context().add_class("card");

    // Toggle buttons for functions (use emoji for simple icons)
    let btn_rotate = ToggleButton::with_label("🔁 Rotate PDF");
    let btn_split = ToggleButton::with_label("📐 Split PDF");
    let btn_merge = ToggleButton::with_label("🧩 Merge PDFs");
    btn_rotate.style_context().add_class("func-button");
    btn_split.style_context().add_class("func-button");
    btn_merge.style_context().add_class("func-button");
    btn_rotate.set_size_request(140, 48);
    btn_split.set_size_request(140, 48);
    btn_merge.set_size_request(140, 48);
    transform_box.append(&btn_rotate);
    transform_box.append(&btn_split);
    transform_box.append(&btn_merge);

    let btn_extract = ToggleButton::with_label("📄 Extract Text");
    let btn_images = ToggleButton::with_label("🖼️ Convert to Images");
    let btn_compress = ToggleButton::with_label("🗜️ Compress PDF");
    btn_extract.style_context().add_class("func-button");
    btn_images.style_context().add_class("func-button");
    btn_compress.style_context().add_class("func-button");
    btn_extract.set_size_request(140, 48);
    btn_images.set_size_request(140, 48);
    btn_compress.set_size_request(140, 48);
    export_box.append(&btn_extract);
    export_box.append(&btn_images);
    export_box.append(&btn_compress);

    vbox.append(&transform_frame);
    vbox.append(&export_frame);

    // Output options
    let output_box = GtkBox::builder().orientation(Orientation::Horizontal).spacing(8).build();
    let same_location = CheckButton::with_label("Use input folder for output");
    same_location.set_active(true);
    let choose_folder_btn = Button::with_label("Choose folder");
    choose_folder_btn.set_sensitive(false);
    output_box.append(&same_location);
    output_box.append(&choose_folder_btn);
    vbox.append(&output_box);

    let filename_entry = Entry::new();
    filename_entry.set_placeholder_text(Some("Output filename (e.g. output.pdf)"));
    vbox.append(&filename_entry);

    let rotate_box = GtkBox::builder().orientation(Orientation::Horizontal).spacing(6).build();
    let rot_90_cw = ToggleButton::with_label("90° CW");
    let rot_90_ccw = ToggleButton::with_label("90° CCW");
    let rot_180 = ToggleButton::with_label("180°");
    rot_90_cw.set_active(true);
    rotate_box.append(&rot_90_cw);
    rotate_box.append(&rot_90_ccw);
    rotate_box.append(&rot_180);
    rotate_box.set_visible(false);
    vbox.append(&rotate_box);
    let pages_entry = Entry::new();
    pages_entry.set_placeholder_text(Some("Pages (e.g. 1-3,5) — empty = all"));
    rotate_box.append(&pages_entry);

    let preview_image = gtk::Image::new();
    preview_image.set_pixel_size(160);
    preview_image.style_context().add_class("preview-image");
    vbox.append(&preview_image);

    let PREVIEW_GENERATOR: Rc<RefCell<Option<std::boxed::Box<dyn Fn() + 'static>>>> = Rc::new(RefCell::new(None));
    let REBUILD_WORKPLACE: Rc<RefCell<Option<std::boxed::Box<dyn Fn() + 'static>>>> = Rc::new(RefCell::new(None));

    {
        let files_for_preview = files.clone();
        let preview_image_cl = preview_image.clone();
        let rot_90_cw_pre = rot_90_cw.clone();
        let rot_90_ccw_pre = rot_90_ccw.clone();
        let rot_180_pre = rot_180.clone();
        let gen = move || {
            let current_files = files_for_preview.borrow().clone();
            if current_files.is_empty() {
                preview_image_cl.clear();
                return;
            }
            let input = current_files[0].clone();
            let degrees = if rot_90_cw_pre.is_active() {
                90
            } else if rot_90_ccw_pre.is_active() {
                -90
            } else if rot_180_pre.is_active() {
                180
            } else {
                0
            };

            let ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis();
            let tmp_prefix = std::env::temp_dir().join(format!("pdf_preview_{}", ts));
            let tmp_png = tmp_prefix.with_extension("png");

            let (sender, receiver) = glib::MainContext::channel::<Result<std::path::PathBuf, String>>(glib::PRIORITY_DEFAULT);

            std::thread::spawn(move || {
                // render first page to PNG from the original PDF using pdftoppm
                if let Ok(_) = std::process::Command::new("pdftoppm").arg("-v").status() {
                    let mut cmd = std::process::Command::new("pdftoppm");
                    cmd.args(&["-png", "-f", "1", "-singlefile", input.to_string_lossy().as_ref(), tmp_prefix.to_string_lossy().as_ref()]);
                    match cmd.status() {
                        Ok(s) if s.success() => {
                            if degrees != 0 {
                                if let Ok(_) = std::process::Command::new("convert").arg("-version").status() {
                                    let deg_arg = degrees.to_string();
                                    let mut rcmd = std::process::Command::new("convert");
                                    rcmd.args(&[tmp_png.to_string_lossy().as_ref(), "-rotate", &deg_arg, tmp_png.to_string_lossy().as_ref()]);
                                    match rcmd.status() {
                                        Ok(rs) if rs.success() => { let _ = sender.send(Ok(tmp_png)); }
                                        Ok(rs) => { let _ = sender.send(Err(format!("convert exited: {}", rs))); }
                                        Err(e) => { let _ = sender.send(Err(format!("failed to spawn convert: {}", e))); }
                                    }
                                } else {
                                    let _ = sender.send(Ok(tmp_png));
                                }
                            } else {
                                let _ = sender.send(Ok(tmp_png));
                            }
                        }
                        Ok(s) => { let _ = sender.send(Err(format!("pdftoppm exited: {}", s))); }
                        Err(e) => { let _ = sender.send(Err(format!("failed to spawn pdftoppm: {}", e))); }
                    }
                } else {
                    let _ = sender.send(Err("pdftoppm not found".into()));
                }
            });

            let preview_image_for_attach = preview_image_cl.clone();
            receiver.attach(None, move |res| {
                match res {
                    Ok(path) => { preview_image_for_attach.set_from_file(Some(&path)); }
                    Err(_) => { preview_image_for_attach.clear(); }
                }
                glib::Continue(false)
            });
        };
        PREVIEW_GENERATOR.borrow_mut().replace(std::boxed::Box::new(gen));
    }

    let workplace = Frame::builder().label("Workplace").build();
    let wp_box = GtkBox::builder().orientation(Orientation::Vertical).spacing(6).margin_top(6).margin_bottom(6).build();
    workplace.set_child(Some(&wp_box));

    let wp_input = Label::new(None);
    let wp_action = Label::new(None);
    let wp_status = Label::new(None);
    let open_output_btn = Button::with_label("Open output folder");
    open_output_btn.set_sensitive(false);

    wp_box.append(&wp_input);
    wp_input.set_visible(false);
    wp_box.append(&wp_action);
    wp_box.append(&wp_status);

    let scroller = gtk::ScrolledWindow::builder().vexpand(true).min_content_height(120).build();
    let file_listbox = gtk::ListBox::new();
    scroller.set_child(Some(&file_listbox));
    wp_box.append(&scroller);

    let files_progress: Rc<RefCell<Vec<ProgressBar>>> = Rc::new(RefCell::new(Vec::new()));
    let files_output_labels: Rc<RefCell<Vec<Label>>> = Rc::new(RefCell::new(Vec::new()));

    wp_box.append(&open_output_btn);

    vbox.append(&workplace);

    let rebuild_workplace = {
        let file_listbox = file_listbox.clone();
        let files = files.clone();
        let files_progress = files_progress.clone();
        let files_output_labels = files_output_labels.clone();
        let filename_entry = filename_entry.clone();
        Box::new(move || {
            while let Some(row) = file_listbox.row_at_index(0) {
                file_listbox.remove(&row);
            }
            files_progress.borrow_mut().clear();
            files_output_labels.borrow_mut().clear();

            for p in files.borrow().iter() {
                let row = GtkBox::builder().orientation(Orientation::Horizontal).spacing(8).margin_top(4).margin_bottom(4).build();
                let in_lbl = Label::new(None);
                let in_name = p.file_name().and_then(|s| s.to_str()).unwrap_or("").to_string();
                let max_in = 28;
                if in_name.chars().count() > max_in {
                    let short: String = in_name.chars().take(max_in - 1).collect();
                    in_lbl.set_label(&format!("{}…", short));
                    in_lbl.set_tooltip_text(Some(&in_name));
                } else {
                    in_lbl.set_label(&in_name);
                    in_lbl.set_tooltip_text(None);
                }
                in_lbl.set_xalign(0.0);
                in_lbl.set_hexpand(true);

                let bar = ProgressBar::new();
                bar.set_fraction(0.0);
                bar.set_hexpand(true);
                bar.set_valign(gtk::Align::Center);

                let out_lbl = Label::new(None);
                let out_full = filename_entry.text().as_str().to_string();
                let max_out = 36;
                if out_full.chars().count() > max_out {
                    let short: String = out_full.chars().skip(out_full.chars().count() - (max_out - 1)).collect();
                    out_lbl.set_label(&format!("…{}", short));
                    out_lbl.set_tooltip_text(Some(&out_full));
                } else {
                    out_lbl.set_label(&out_full);
                    out_lbl.set_tooltip_text(None);
                }
                out_lbl.set_xalign(1.0);
                out_lbl.set_hexpand(true);

                row.append(&in_lbl);
                row.append(&bar);
                row.append(&out_lbl);

                file_listbox.append(&row);
                files_progress.borrow_mut().push(bar);
                files_output_labels.borrow_mut().push(out_lbl);
            }
        })
    };

    // initial build (empty)
    rebuild_workplace();
    // store rebuild into shared holder so earlier closures can call it
    REBUILD_WORKPLACE.borrow_mut().replace(rebuild_workplace);

    let run_button = Button::with_label("Run");
    vbox.append(&run_button);

    // Header bar: app icon, title, spacer, quick actions
    let header_box = GtkBox::builder().orientation(Orientation::Horizontal).spacing(8).build();
    let app_icon = gtk::Image::builder().icon_name("application-pdf-symbolic").build();
    let title_lbl = Label::builder().label("PDF Toolset").build();
    let spacer = GtkBox::builder().orientation(Orientation::Horizontal).build();
    spacer.set_hexpand(true);
    let header_add = Button::with_label("＋");
    let header_run = Button::with_label("▶");
    header_box.append(&app_icon);
    header_box.append(&title_lbl);
    header_box.append(&spacer);
    header_box.append(&header_add);
    header_box.append(&header_run);

    let add_files_btn_cl = add_files_btn.clone();
    header_add.connect_clicked(move |_| {
        add_files_btn_cl.emit_clicked();
    });
    let run_button_cl = run_button.clone();
    header_run.connect_clicked(move |_| {
        run_button_cl.emit_clicked();
    });

    let window = ApplicationWindow::builder()
        .application(app)
        .title("PDF Toolset")
        .default_width(480)
        .default_height(300)
        .child(&vbox)
        .build();

    vbox.prepend(&header_box);

    let files_clone = files.clone();
    let file_list_label_clone = file_list_label.clone();
    let wp_input_clone = wp_input.clone();
    let file_pool_box_clone = file_pool_box.clone();
    let filename_entry_for_drop = filename_entry.clone();
    let file_checks: Rc<RefCell<Vec<gtk::CheckButton>>> = Rc::new(RefCell::new(Vec::new()));
    let file_checks_for_drop = file_checks.clone();
    let drop_target = gtk::DropTarget::new(GlibType::STRING, gdk::DragAction::COPY);
    let preview_gen_for_drop = PREVIEW_GENERATOR.clone();
    let rebuild_workplace_for_drop = REBUILD_WORKPLACE.clone();
    drop_target.connect_drop(move |_, value, _, _| {
        if let Ok(uri) = value.get::<String>() {
            for raw in uri.split('\n') {
                if raw.trim().is_empty() {
                    continue;
                }
                let p = raw.trim().trim_start_matches("file://");
                let path = std::path::PathBuf::from(p);
                files_clone.borrow_mut().push(path);
            }
                let names: Vec<String> = files_clone
                .borrow()
                .iter()
                .map(|p| p.file_name().and_then(|s| s.to_str()).unwrap_or("").to_string())
                .collect();
            file_list_label_clone.set_label(&names.join(", "));
            wp_input_clone.set_label(&format!("Input: {}", names.join(", ")));
            for cb in file_checks_for_drop.borrow().iter() {
                    file_pool_box_clone.remove(cb);
                }
                file_checks_for_drop.borrow_mut().clear();
                for p in files_clone.borrow().iter() {
                    let cb = gtk::CheckButton::with_label(p.file_name().and_then(|s| s.to_str()).unwrap_or(""));
                    file_pool_box_clone.append(&cb);
                    file_checks_for_drop.borrow_mut().push(cb);
                }
                if let Some(first) = files_clone.borrow().get(0) {
                    if let Some(stem) = first.file_stem().and_then(|s| s.to_str()) {
                        filename_entry_for_drop.set_text(&format!("{}_opt.pdf", stem));
                    }
                }
                if let Some(gen) = &*preview_gen_for_drop.borrow() {
                    (gen)();
                }
                if let Some(rb) = &*rebuild_workplace_for_drop.borrow() {
                    (rb)();
                }
        }
        true
    });
    drop_frame.add_controller(drop_target);

    // Add files button -> use zenity to pick multiple files
    let files_clone_for_add = files.clone();
    let file_list_label_for_add = file_list_label.clone();
    let wp_input_for_add = wp_input.clone();
    let file_pool_for_add = file_pool_box.clone();
    let file_checks_for_add = file_checks.clone();
    let filename_entry_for_add = filename_entry.clone();
    let preview_gen_for_add = PREVIEW_GENERATOR.clone();
    let rebuild_workplace_for_add = REBUILD_WORKPLACE.clone();
    add_files_btn.connect_clicked(move |_| {
        if let Ok(out) = std::process::Command::new("zenity").arg("--file-selection").arg("--multiple").arg("--separator=\n").output() {
            if out.status.success() {
                let text = String::from_utf8_lossy(&out.stdout).to_string();
                for line in text.split('\n') {
                    let line = line.trim();
                    if line.is_empty() { continue; }
                    files_clone_for_add.borrow_mut().push(PathBuf::from(line));
                }
                let names: Vec<String> = files_clone_for_add
                    .borrow()
                    .iter()
                    .map(|p| p.file_name().and_then(|s| s.to_str()).unwrap_or("").to_string())
                    .collect();
                file_list_label_for_add.set_label(&names.join(", "));
                wp_input_for_add.set_label(&format!("Input: {}", names.join(", ")));
                // update pool UI: remove existing checkbuttons then recreate
                for cb in file_checks_for_add.borrow().iter() {
                    file_pool_for_add.remove(cb);
                }
                file_checks_for_add.borrow_mut().clear();
                for p in files_clone_for_add.borrow().iter() {
                    let cb = gtk::CheckButton::with_label(p.file_name().and_then(|s| s.to_str()).unwrap_or(""));
                    file_pool_for_add.append(&cb);
                    file_checks_for_add.borrow_mut().push(cb);
                }
                if let Some(first) = files_clone_for_add.borrow().get(0) {
                    if let Some(stem) = first.file_stem().and_then(|s| s.to_str()) {
                        filename_entry_for_add.set_text(&format!("{}_opt.pdf", stem));
                    }
                }
                if let Some(gen) = &*preview_gen_for_add.borrow() {
                    (gen)();
                }
                if let Some(rb) = &*rebuild_workplace_for_add.borrow() {
                    (rb)();
                }
            }
        }
    });

    // Select All / Remove Selected behavior for the pool
    let file_checks_for_select_all = file_checks.clone();
    select_all_btn.connect_clicked(move |_| {
        for cb in file_checks_for_select_all.borrow().iter() {
            cb.set_active(true);
        }
    });

    let files_for_remove = files.clone();
    let file_checks_for_remove = file_checks.clone();
    let file_pool_for_remove = file_pool_box.clone();
    let file_list_label_for_remove = file_list_label.clone();
    let wp_input_for_remove = wp_input.clone();
    let preview_gen_for_remove = PREVIEW_GENERATOR.clone();
    let filename_entry_for_remove = filename_entry.clone();
    let rebuild_workplace_for_remove = REBUILD_WORKPLACE.clone();
    remove_selected_btn.connect_clicked(move |_| {
        let mut to_remove: Vec<usize> = Vec::new();
        for (i, cb) in file_checks_for_remove.borrow().iter().enumerate() {
            if cb.is_active() {
                to_remove.push(i);
            }
        }
        if to_remove.is_empty() {
            return;
        }
        for &i in to_remove.iter().rev() {
            if let Some(cb) = file_checks_for_remove.borrow_mut().get(i) {
                file_pool_for_remove.remove(cb);
            }
            file_checks_for_remove.borrow_mut().remove(i);
            files_for_remove.borrow_mut().remove(i);
        }
        let names: Vec<String> = files_for_remove
            .borrow()
            .iter()
            .map(|p| p.file_name().and_then(|s| s.to_str()).unwrap_or("").to_string())
            .collect();
        file_list_label_for_remove.set_label(&names.join(", "));
        wp_input_for_remove.set_label(&format!("Input: {}", names.join(", ")));
        if let Some(first) = files_for_remove.borrow().get(0) {
            if let Some(stem) = first.file_stem().and_then(|s| s.to_str()) {
                filename_entry_for_remove.set_text(&format!("{}_opt.pdf", stem));
            }
        } else {
            filename_entry_for_remove.set_text("");
        }
        if let Some(gen) = &*preview_gen_for_remove.borrow() {
            (gen)();
        }
        if let Some(rb) = &*rebuild_workplace_for_remove.borrow() {
            (rb)();
        }
    });

    let selected_action: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
    let selected_action_clone = selected_action.clone();

    let output_dir: Rc<RefCell<Option<PathBuf>>> = Rc::new(RefCell::new(None));
    let output_dir_clone_for_choose = output_dir.clone();

    let toggles: Vec<ToggleButton> = vec![btn_rotate.clone(), btn_split.clone(), btn_merge.clone(), btn_extract.clone(), btn_images.clone(), btn_compress.clone()];
        let toggles_rc = Rc::new(toggles);
        let filename_entry_for_toggles = filename_entry.clone();
        let files_for_toggles = files.clone();

    for tb in toggles_rc.iter() {
        let toggles_inner = toggles_rc.clone();
        let sel_clone = selected_action_clone.clone();
        let wp_action_clone = wp_action.clone();
        let filename_entry_local = filename_entry_for_toggles.clone();
        let files_local = files_for_toggles.clone();
        tb.connect_toggled(move |t| {
            if t.is_active() {
                for other in toggles_inner.iter() {
                    if other != t {
                        other.set_active(false);
                    }
                }
                let label_str = t.label().map(|s| s.to_string()).unwrap_or_default();
                sel_clone.borrow_mut().replace(label_str.clone());
                wp_action_clone.set_label(&format!("Queued: {}", label_str));
                if label_str.contains("Split") {
                    if let Some(first) = files_local.borrow().get(0) {
                        if let Some(stem) = first.file_stem().and_then(|s| s.to_str()) {
                            filename_entry_local.set_text(&format!("{}-pages", stem));
                        }
                    }
                } else if label_str.contains("Images") {
                    if let Some(first) = files_local.borrow().get(0) {
                        if let Some(stem) = first.file_stem().and_then(|s| s.to_str()) {
                            filename_entry_local.set_text(&format!("{}-images", stem));
                        }
                    }
                }
            } else {
                let any = toggles_inner.iter().any(|b| b.is_active());
                if !any {
                    sel_clone.borrow_mut().take();
                    wp_action_clone.set_label("");
                }
            }
        });
    }

    let rotate_modes: Vec<ToggleButton> = vec![rot_90_cw.clone(), rot_90_ccw.clone(), rot_180.clone()];
    let rotate_modes_rc = Rc::new(rotate_modes);
    for rm in rotate_modes_rc.iter() {
        let others = rotate_modes_rc.clone();
        rm.connect_toggled(move |b| {
            if b.is_active() {
                for o in others.iter() {
                    if o != b {
                        o.set_active(false);
                    }
                }
            }
        });
    }

    let preview_gen_cl = PREVIEW_GENERATOR.clone();
    let files_for_preview_toggle = files.clone();
    let btn_rotate_clone_for_preview = btn_rotate.clone();
    for r in vec![rot_90_cw.clone(), rot_90_ccw.clone(), rot_180.clone()].into_iter() {
        let gen_clone = preview_gen_cl.clone();
        let files_local = files_for_preview_toggle.clone();
        let rotate_active = btn_rotate_clone_for_preview.clone();
        r.connect_toggled(move |b| {
            if b.is_active() && rotate_active.is_active() && !files_local.borrow().is_empty() {
                if let Some(gen) = &*gen_clone.borrow() {
                    gen();
                }
            }
        });
    }

    // Update compress_box and rotate_box visibility when relevant function toggles change
    // no compress options to toggle

    let rotate_box_cl = rotate_box.clone();
    let btn_rotate_cl = btn_rotate.clone();
    btn_rotate_cl.connect_toggled(move |b| {
        rotate_box_cl.set_visible(b.is_active());
    });

    choose_folder_btn.connect_clicked(move |_| {
        if let Ok(out) = std::process::Command::new("zenity").arg("--file-selection").arg("--directory").output() {
            if out.status.success() {
                let path = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if !path.is_empty() {
                    output_dir_clone_for_choose.borrow_mut().replace(PathBuf::from(path));
                }
            }
        }
    });

    let choose_folder_btn_clone = choose_folder_btn.clone();
    same_location.connect_toggled(move |c| {
        choose_folder_btn_clone.set_sensitive(!c.is_active());
    });

    let files_for_run = files.clone();
    let window_for_run = window.clone();
    let selected_action_for_run = selected_action.clone();
    let output_dir_for_run = output_dir.clone();
    let filename_entry_for_run = filename_entry.clone();
    let wp_status_clone = wp_status.clone();
    let wp_input_for_run = wp_input.clone();
    let open_output_btn_clone = open_output_btn.clone();
    let files_progress_for_run = files_progress.clone();
    let files_output_labels_for_run = files_output_labels.clone();
    let same_location_for_run = same_location.clone();

    let run_btn_for_connect = run_button.clone();
    let run_btn_for_action = run_button.clone();
    let pages_entry_for_run = pages_entry.clone();
    let rot_90_cw_cl = rot_90_cw.clone();
    let rot_90_ccw_cl = rot_90_ccw.clone();
    let rot_180_cl = rot_180.clone();

    run_btn_for_connect.connect_clicked(move |_| {
        let current_files = files_for_run.borrow().clone();
        if current_files.is_empty() {
            let dialog = gtk::MessageDialog::builder()
                .transient_for(&window_for_run)
                .modal(true)
                .text("No files dropped. Please drag PDF files into the drop area.")
                .build();
            dialog.add_button("OK", gtk::ResponseType::Ok);
            dialog.connect_response(|d, _| {
                d.close();
            });
            dialog.show();
            return;
        }

        // update workplace input label
        let names: Vec<String> = current_files.iter().map(|p| p.file_name().and_then(|s| s.to_str()).unwrap_or("").to_string()).collect();
        wp_input_for_run.set_label(&format!("Input: {}", names.join(", ")));

        // determine output dir
        let out_dir = if same_location_for_run.is_active() {
            current_files.get(0).and_then(|p| p.parent().map(|pp| pp.to_path_buf()))
        } else {
            output_dir_for_run.borrow().clone()
        };
        let out_dir_display = out_dir.as_ref().map(|p| p.to_string_lossy().to_string()).unwrap_or_else(||"(not set)".to_string());
        wp_status_clone.set_label(&format!("Output folder: {}", out_dir_display));

        // output filename
        let out_name = filename_entry_for_run.text().as_str().to_string();
        if out_name.is_empty() {
            wp_status_clone.set_label(&format!("Output folder: {} — missing filename", out_dir_display));
            return;
        }

        // selected action
        let action = selected_action_for_run.borrow().clone();
        if action.is_none() {
            let dialog = gtk::MessageDialog::builder()
                .transient_for(&window_for_run)
                .modal(true)
                .text("No action selected. Please choose a function block.")
                .build();
            dialog.add_button("OK", gtk::ResponseType::Ok);
            dialog.connect_response(|d, _| {
                d.close();
            });
            dialog.show();
            return;
        }

        // prepare UI and background execution
        let action = selected_action_for_run.borrow().clone().unwrap_or_default();
        let short_action = action.clone();
        // set brief status for user
        wp_status_clone.set_label(&format!("Running: {}", short_action));
        run_btn_for_action.set_sensitive(false);

        // compress mode removed; backend uses Standard by default

        // decide rotation degrees (capture UI state on main thread only)
        let rotation_degrees_value = if action.contains("Rotate") {
            if rot_90_cw_cl.is_active() {
                90
            } else if rot_90_ccw_cl.is_active() {
                -90
            } else if rot_180_cl.is_active() {
                180
            } else {
                90
            }
        } else {
            0
        };

        // start pulsing per-file progress bars on main context
        let pulse_id: Rc<RefCell<Option<glib::SourceId>>> = Rc::new(RefCell::new(None));
        let files_progress_for_pulse = files_progress_for_run.clone();
        let pulse_id_clone = pulse_id.clone();
        let id = glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
            for b in files_progress_for_pulse.borrow().iter() { b.pulse(); }
            glib::Continue(true)
        });
        *pulse_id_clone.borrow_mut() = Some(id);

        // spawn background thread to run operation; use a glib channel to update UI safely
        let out_dir_clone = out_dir.clone();
        let filename = filename_entry_for_run.text().to_string();
        let files_for_thread = current_files.clone();
        let (sender, receiver) = glib::MainContext::channel::<Result<(), String>>(glib::PRIORITY_DEFAULT);

        // attach receiver on main context to update UI (stop pulsing and mark done)
        let files_progress_for_recv = files_progress_for_run.clone();
        let files_output_labels_for_recv = files_output_labels_for_run.clone();
        let wp_status_after = wp_status_clone.clone();
        let open_btn_after = open_output_btn_clone.clone();
        let run_btn_after = run_btn_for_action.clone();
        let pulse_id_for_recv = pulse_id.clone();
        receiver.attach(None, move |res| {
            // stop pulsing
            if let Some(src) = pulse_id_for_recv.borrow_mut().take() {
                src.remove();
            }
            // mark all per-file bars as complete
            for b in files_progress_for_recv.borrow().iter() { b.set_fraction(1.0); }
            // If success, set status; on error show message
            match res {
                Ok(_) => wp_status_after.set_label("Completed"),
                Err(e) => wp_status_after.set_label(&format!("Error: {}", e)),
            }
            open_btn_after.set_sensitive(true);
            run_btn_after.set_sensitive(true);
            glib::Continue(false)
        });

        let degrees_for_thread = rotation_degrees_value;
        // parse pages entry on main thread to avoid moving GTK objects into the worker thread
        let pages_text_main = pages_entry_for_run.text().as_str().trim().to_string();
        let pages_vec_for_thread: Option<Vec<u32>> = if pages_text_main.is_empty() {
            None
        } else {
            let mut vec: Vec<u32> = Vec::new();
            for part in pages_text_main.split(',') {
                let part = part.trim();
                if part.contains('-') {
                    let mut it = part.splitn(2, '-');
                    if let (Some(a), Some(b)) = (it.next(), it.next()) {
                        if let (Ok(sa), Ok(sb)) = (a.parse::<u32>(), b.parse::<u32>()) {
                            for n in sa..=sb { vec.push(n); }
                        }
                    }
                } else if !part.is_empty() {
                    if let Ok(n) = part.parse::<u32>() { vec.push(n); }
                }
            }
            if vec.is_empty() { None } else { Some(vec) }
        };

        // before running, set per-file output labels to expected outputs (on main thread)
        {
            let outs = files_output_labels_for_run.clone();
            if action.contains("Split") {
                for (i, p) in files_for_thread.iter().enumerate() {
                    if let Some(stem) = p.file_stem().and_then(|s| s.to_str()) {
                        let folder = out_dir_clone.clone().unwrap_or_else(|| p.parent().unwrap().to_path_buf()).join(format!("{}-pages", stem));
                        if let Some(lbl) = outs.borrow().get(i) { lbl.set_label(&folder.to_string_lossy()); }
                    }
                }
            } else if action.contains("Images") {
                for (i, p) in files_for_thread.iter().enumerate() {
                    if let Some(stem) = p.file_stem().and_then(|s| s.to_str()) {
                        let folder = out_dir_clone.clone().unwrap_or_else(|| p.parent().unwrap().to_path_buf()).join(format!("{}-images", stem));
                        if let Some(lbl) = outs.borrow().get(i) { lbl.set_label(&folder.to_string_lossy()); }
                    }
                }
            } else {
                // single output filename — set for all rows
                let common = out_dir_clone.clone().unwrap_or_else(|| files_for_thread[0].parent().unwrap().to_path_buf()).join(&filename);
                for lbl in outs.borrow().iter() { lbl.set_label(&common.to_string_lossy()); }
            }
        }

        std::thread::spawn(move || {
            let result = match action.as_str() {
                a if a.contains("Merge") => {
                    let out = out_dir_clone.clone().unwrap_or_else(|| files_for_thread[0].parent().unwrap().to_path_buf()).join(&filename);
                    backend::merge(&files_for_thread, &out)
                }
                a if a.contains("Split") => {
                    let input = files_for_thread.get(0).cloned().unwrap();
                    let outdir = out_dir_clone.clone().unwrap_or_else(|| input.parent().unwrap().to_path_buf());
                    backend::split(&input, &outdir)
                }
                a if a.contains("Extract") => {
                    let input = files_for_thread.get(0).cloned().unwrap();
                    let out = out_dir_clone.clone().unwrap_or_else(|| input.parent().unwrap().to_path_buf()).join(&filename);
                    backend::extract_text(&input, &out)
                }
                a if a.contains("Images") || a.contains("Images") => {
                    let input = files_for_thread.get(0).cloned().unwrap();
                    let outdir = out_dir_clone.clone().unwrap_or_else(|| input.parent().unwrap().to_path_buf());
                    // default to png
                    backend::convert_to_images(&input, &outdir, "png")
                }
                a if a.contains("Compress") => {
                    let input = files_for_thread.get(0).cloned().unwrap();
                    let out = out_dir_clone.clone().unwrap_or_else(|| input.parent().unwrap().to_path_buf()).join(&filename);
                    backend::compress(&input, &out)
                }
                a if a.contains("Rotate") => {
                    let input = files_for_thread.get(0).cloned().unwrap();
                    let outdir = out_dir_clone.clone().unwrap_or_else(|| input.parent().unwrap().to_path_buf());
                    let out = outdir.join(&filename);
                    backend::rotate(&input, &out, degrees_for_thread, pages_vec_for_thread.clone())
                }
                _ => Err("Unsupported action or not implemented".into()),
            };
            let _ = sender.send(result);
        });
    });

    // Open output folder button — resolve folder the same way Run does
    let output_dir_for_open2 = output_dir.clone();
    let files_for_open = files.clone();
    let same_location_for_open = same_location.clone();
    let window_for_open = window.clone();
    open_output_btn.connect_clicked(move |_| {
        // Determine folder: if "use input folder" is active, open parent of first input,
        // otherwise open the chosen output_dir (if set).
        let folder_opt: Option<PathBuf> = if same_location_for_open.is_active() {
            files_for_open
                .borrow()
                .get(0)
                .and_then(|p| p.parent().map(|pp| pp.to_path_buf()))
        } else {
            output_dir_for_open2.borrow().clone()
        };

        if let Some(folder) = folder_opt {
            if folder.exists() {
                match std::process::Command::new("xdg-open").arg(&folder).spawn() {
                    Ok(_) => {},
                    Err(e) => {
                        let dialog = gtk::MessageDialog::builder()
                            .transient_for(&window_for_open)
                            .modal(true)
                            .text(&format!("Failed to open folder: {}", e))
                            .build();
                        dialog.add_button("OK", gtk::ResponseType::Ok);
                        dialog.connect_response(|d, _| { d.close(); });
                        dialog.show();
                    }
                }
            } else {
                let dialog = gtk::MessageDialog::builder()
                    .transient_for(&window_for_open)
                    .modal(true)
                    .text(&format!("Output folder does not exist: {}", folder.to_string_lossy()))
                    .build();
                dialog.add_button("OK", gtk::ResponseType::Ok);
                dialog.connect_response(|d, _| { d.close(); });
                dialog.show();
            }
        } else {
            let dialog = gtk::MessageDialog::builder()
                .transient_for(&window_for_open)
                .modal(true)
                .text("Output folder not set")
                .build();
            dialog.add_button("OK", gtk::ResponseType::Ok);
            dialog.connect_response(|d, _| { d.close(); });
            dialog.show();
        }
    });

    window.present();
}
