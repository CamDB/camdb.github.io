use std::{fs::{self, File}, io::{BufWriter, Write}, path::Path, sync::mpsc, thread, time::{Duration, Instant}};
use lazy_static::lazy_static;

use anyhow::Result;
use notify::{RecursiveMode, Watcher};

mod posts;
mod templating;

lazy_static! {
    pub static ref OUTPUT_PATH: String = {
        if cfg!(debug_assertions) {
            "debug".to_string()
        } else {
            "docs".to_string()
        }
    };
}

fn main() -> Result<()> {
    env_logger::init();

    recompile_html()?;

    let (tx, rx) = mpsc::channel::<notify::Result<notify::Event>>();

    // Use recommended_watcher() to automatically select the best implementation
    // for your platform. The `EventHandler` passed to this constructor can be a
    // closure, a `std::sync::mpsc::Sender`, a `crossbeam_channel::Sender`, or
    // another type the trait is implemented for.
    let mut watcher = notify::recommended_watcher(tx)?;

    // Add a path to be watched. All files and directories at that path and
    // below will be monitored for changes.
    watcher.watch(Path::new("posts"), RecursiveMode::Recursive)?;

    let mut last_recompile = Instant::now();
    loop {
        if let Ok(Ok(event)) = rx.try_recv() {
            match event.kind {
                notify::EventKind::Create(_) | notify::EventKind::Modify(_) | notify::EventKind::Remove(_) => {
                    if last_recompile.elapsed() > Duration::from_secs(1) {
                        last_recompile = Instant::now();
                        thread::sleep(Duration::from_secs_f32(0.1));
                        recompile_html()?;
                    }
                },
                _ => (),
            }
        }
    }
}

fn recompile_html() -> Result<()> {
    log::info!("Recompiling html...");

    const OUTPUT_FOLDER: &str = if cfg!(debug_assertions) {
        "debug"
    } else {
        "docs"
    };

    // Fetch all posts from the posts dir.
    let posts = posts::get_all_posts(Path::new("./posts"))?;

    log::info!("Got {} post(s)", posts.len());

    // Clean existing output folder.
    fs::remove_dir_all(&format!("{}/", OUTPUT_FOLDER)).ok();

    // Generate the homepage.

    // Generate pages for each post.
    for post in posts {
        let post_html = templating::post_page(&post)?;

        let output_path = format!("{}/post/{}.html", OUTPUT_FOLDER, post.metadata.normalised_title);
        let output_path = Path::new(&output_path);

        fs::create_dir_all(&output_path.parent().unwrap())?;
        let file = File::create_new(&output_path)?;
        let mut writer = BufWriter::new(file);
        writer.write(post_html.as_bytes())?;
        writer.flush()?;
    }

    // Save HTML to appropriate locations.

    Ok(())
}
