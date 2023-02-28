#![allow(deprecated)]
use std::thread;
use std::fs;
use std::rc::Rc;
use std::process::Command;

use rfd::FileDialog;
use directories::{ProjectDirs};
use image::{GenericImage, ImageBuffer, RgbImage, DynamicImage};
use image::imageops::FilterType;
use lazy_static::lazy_static;
use serde::{Serialize, Deserialize};
use slint::{Model, StandardListViewItem, VecModel};
slint::include_modules!();

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Config {
    rows_count: u32,
    columns_count: u32,
    spacing: u32,
    thumb_width: u32
}

impl ::std::default::Default for Config {
    fn default() -> Self {
        Self {
            rows_count: 4,
            columns_count: 4,
            spacing: 10,
            thumb_width: 500
        }
    }
}

lazy_static! {
    static ref CONFIG_FILE: String = {
        let proj_dirs = ProjectDirs::from("", "",  "kagura").unwrap();
        let config_file = proj_dirs.config_dir().join("kagura.toml");
        config_file.display().to_string()
    };
}

struct VideoFile {
    path: String,
    width: u32,
    height: u32,
    duration: f32
}

fn main() {
    let ui = AppWindow::new();
    let model = Rc::new(VecModel::from(vec![]));
    let config: Config = confy::load_path(CONFIG_FILE.clone()).unwrap_or(Config::default());

    // set slint widgets values
    ui.set_rowsCount(slint::SharedString::from(config.rows_count.to_string()));
    ui.set_columnsCount(slint::SharedString::from(config.columns_count.to_string()));
    ui.set_spacing(slint::SharedString::from(config.spacing.to_string()));
    ui.set_thumbWidth(slint::SharedString::from(config.thumb_width.to_string()));

    let ui_handle = ui.as_weak();
    ui.on_save_config(move || {
        let ui = ui_handle.unwrap();
        
        let main_config: Config = match confy::load_path(CONFIG_FILE.clone()) {
            Ok(config) => config,
            Err(_e) => Config::default()
        };

        let mut config = main_config.clone();
        config.rows_count = ui.get_rowsCount().parse::<u32>().unwrap_or(config.rows_count);
        config.columns_count = ui.get_columnsCount().parse::<u32>().unwrap_or(config.columns_count);
        config.spacing = ui.get_spacing().parse::<u32>().unwrap_or(config.spacing);
        config.thumb_width = ui.get_thumbWidth().parse::<u32>().unwrap_or(config.thumb_width);

        match confy::store_path(CONFIG_FILE.clone(), config) {
            Ok(_) => (),
            Err(e) => println!("{:#?}", e)
        };
    });

    let ui_handle = ui.as_weak();
    ui.on_open_file_dialog(move || {
        let ui = ui_handle.unwrap();
        let _file = FileDialog::new()
            .add_filter("video", &["mkv", "mp4"])
            .set_directory("/")
            .pick_file();
        let selected_file = _file.unwrap().into_os_string().into_string().unwrap().into();

        model.push(StandardListViewItem { text: selected_file });
        ui.set_filesModel(model.clone().into()); 
    });

    let ui_handle = ui.as_weak();
    ui.on_run(move || {
        let ui = ui_handle.unwrap();
        let model = ui.get_filesModel().clone();
        for item in model.iter() {
            let path: String = item.text.into();
            let duration: f32 = get_video_duration(&path);
            let width: u32 = get_video_width(&path);
            let height: u32 = get_video_height(&path);

            thread::spawn(move || {
                create_thumbnails(&VideoFile {path, width, height, duration});
            });
        };
    });

    ui.run();
}

fn get_video_duration(file: &String) -> f32 {

    let command = format!("ffprobe -v error -show_entries format=duration -of default=noprint_wrappers=1:nokey=1 \"{file}\"");
    let output = Command::new("sh")
        .arg("-c")
        .arg(command.clone())
        .output()
        .expect("failed to execute process");

    let duration: f32 = String::from_utf8_lossy(&output.stdout).trim().parse::<f32>().unwrap();

    return duration;
}

fn get_video_width(file: &String) -> u32 {

    let command = format!("ffprobe -v error -select_streams v:0 -show_entries stream=width -of default=noprint_wrappers=1:nokey=1 \"{file}\"");
    let output = Command::new("sh")
        .arg("-c")
        .arg(command.clone())
        .output()
        .expect("failed to execute process");

    let width: u32 = String::from_utf8_lossy(&output.stdout).trim().parse::<u32>().unwrap();

    return width;
}

fn get_video_height(file: &String) -> u32 {

    let command = format!("ffprobe -v error -select_streams v:0 -show_entries stream=height -of default=noprint_wrappers=1:nokey=1 \"{file}\"");
    let output = Command::new("sh")
        .arg("-c")
        .arg(command.clone())
        .output()
        .expect("failed to execute process");

    let height: u32 = String::from_utf8_lossy(&output.stdout).trim().parse::<u32>().unwrap();

    return height;
}

fn create_thumbnails(file: &VideoFile) {

    let config: Config = match confy::load_path(CONFIG_FILE.clone()) {
        Ok(config) => config,
        Err(_e) => Config::default()
    };

    let columns: u32 = config.columns_count;
    let rows: u32 = config.rows_count;
    let thumb_width: u32 = config.thumb_width;
    let spacing: u32 = config.spacing;
    
    let aspect_ratio: f32 = file.width as f32 / file.height as f32;
    let thumb_height: u32 = (thumb_width as f32 / aspect_ratio) as u32;

    let total_thumbs: u32 = rows * columns;
    let start_time: f32 = file.duration / total_thumbs as f32;

    let proj_dirs = ProjectDirs::from("", "",  "kagura").unwrap();
    let cache_dir = proj_dirs.cache_dir();
    fs::create_dir_all(cache_dir);
    
    let cache_dir = proj_dirs.cache_dir().display();
    for i in 0..total_thumbs {
        let output_path: String = format!("{cache_dir}/thumb-{i}.png");
        let time_position: f32 = start_time * i as f32;
        let file = file.clone();
        let p = String::from(&file.path);
        fs::remove_file(output_path.clone());
        create_thumbnail(p, &output_path, time_position);
    }


    let mut index: u32 = 0;
    let w: u32 = (columns * thumb_width ) + (spacing * columns + spacing);
    let h: u32 = (rows * thumb_height) + (spacing * rows + spacing);
    let mut image: RgbImage = ImageBuffer::new(w, h);
    for i in 0..rows {
        for j in 0..columns {
            let left: u32 = (j * thumb_width)  + ((j + 1) * spacing);
            let top: u32 = (i * thumb_height) + ((i + 1) * spacing);
            let thumb_path: String = format!("{cache_dir}/thumb-{index}.png");
            let thumb_image = image::open(thumb_path.clone()).unwrap();
            let subimg: DynamicImage = thumb_image.resize(thumb_width, thumb_height, FilterType::Nearest);
            println!("copied row {} column {}", i, j);
            image.copy_from(&subimg.into_rgb8(), left, top).expect("poop");
            fs::remove_file(thumb_path).unwrap();
            index += 1;
        }
    }
    image.save(String::from(file.path.clone()) + ".png").unwrap();
    println!("finished!");

}

fn create_thumbnail(path: String, output_path: &String, time_position: f32) {

    use std::process::{Stdio};
    let command = format!("ffmpeg -ss {time_position} -i \"{path}\" -frames:v 1 -qscale:v 3 \"{output_path}\"");
    let child = Command::new("sh")
        .arg("-c")
        .arg(command.clone())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to execute process");

    child.wait_with_output().expect("");

}
