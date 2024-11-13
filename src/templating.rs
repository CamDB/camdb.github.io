use std::{env, fs};

use lazy_static::lazy_static;
use tera::{Context, Tera};

use crate::posts::Post;

lazy_static! {
    pub static ref TEMPLATES: Tera = {
        let mut tera = match Tera::new("templates/**/*") {
            Ok(t) => t,
            Err(e) => {
                println!("Parsing error(s): {}", e);
                ::std::process::exit(1);
            }
        };
        tera.autoescape_on(vec![".html", ".sql"]);
        tera
    };

    pub static ref BASE_PATH: String = {
        if cfg!(debug_assertions) {
            env::current_dir().unwrap().to_string_lossy().to_string()
        } else {
            "".to_string()
        }
    };
}

pub fn post_page(post: &Post) -> Result<String, tera::Error> {
    let mut context = Context::new();
    context.insert("title", &post.metadata.title);
    context.insert("body", &post.body.as_str());
    context.insert("base_path", BASE_PATH.as_str());

    TEMPLATES.render("post.html", &context)
}
