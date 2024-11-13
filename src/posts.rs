use std::{
    fs::{self, File}, io::{BufReader, Read}, path::Path
};

use anyhow::anyhow;
use chrono::Utc;

use kuchiki::traits::*;

#[derive(Debug, PartialEq, Eq)]
pub struct Post {
    pub id: u32,
    pub creation_datetime: chrono::DateTime<Utc>,
    pub body: String,
    pub metadata: PostMetadata,
}

#[derive(Debug, PartialEq, Eq)]
pub struct PostContents {
    pub metadata: PostMetadata,
    pub body: String,
}

#[derive(Debug, PartialEq, Eq)]
pub struct PostMetadata {
    pub title: String,
    pub normalised_title: String,
    pub tags: Vec<String>,
}

fn normalise_title_for_url(title: &str) -> String {
    title
        .replace("-", "")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("_")
        .to_lowercase()
}

fn parse_post(contents: &str) -> anyhow::Result<(String, PostMetadata)> {
    let document = kuchiki::parse_html().one(contents);

    let title = if let Ok(title) = document.select_first("title") {
        title.text_contents()
    } else {
        log::error!("No title found in document");
        log::info!("Document structure: {:?}", document);
        return Err(anyhow!("Post has no title element"))
    };

    let tags = if let Ok(tags) = document.select("tag") {
        tags
            .map(|tag| tag.text_contents())
            .collect()
    } else {
        Vec::new()
    };

    // Get contents
    let body = if let Ok(body) = document.select_first("contents") {
        let node = body.as_node();

        let mut html = Vec::new();
        for child in node.children() {
            child.serialize(&mut html)?;
        }
        String::from_utf8(html)?.trim().to_string()
    } else {
        String::new()
    };

    Ok((body, PostMetadata {
            normalised_title: normalise_title_for_url(&title),
            title,
            tags,
        }))
}

pub fn get_all_posts(path: &Path) -> anyhow::Result<Vec<Post>> {
    let mut post_id = 0;
    let mut html_files = Vec::new();

    if path.is_dir() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                html_files.extend(get_all_posts(&path)?);
            } else if let Some(extension) = path.extension() {
                if extension.eq_ignore_ascii_case("html") || extension.eq_ignore_ascii_case("htm") {
                    let creation_datetime: chrono::DateTime<Utc> = {
                        let f = File::open(&path)?;
                        (&f).metadata()?.created()?.into()
                    };

                    let buf = fs::read_to_string(&path)?;

                    let (body, metadata) = parse_post(&buf)?;

                    html_files.push(Post {
                        id: post_id,
                        creation_datetime,
                        body,
                        metadata,
                    });

                    post_id += 1;
                }
            }
        }
    } else {
        return Err(anyhow::anyhow!("Path {:?} is not a directory", path))
    }

    Ok(html_files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_basic_parsing() -> anyhow::Result<()> {
        let content = r#"
            <metadata>
                <title>Title</title>
                <tags>
                    <tag>Tag A</tag>
                    <tag>Tag B</tag>
                </tags>
            </metadata>
            <contents>
                <p>Hello World</p>
            </contents>
        "#;

        let (body, metadata) = parse_post(content)?;
        assert_eq!(metadata.tags, vec!["Tag A", "Tag B"]);
        assert_eq!(body, "<p>Hello World</p>");
        Ok(())
    }

    #[test]
    fn test_empty_tags() -> anyhow::Result<()> {
        let content = r#"
            <metadata>
                <title>Test</title>
                <tags></tags>
            </metadata>
            <contents>
                <p>Content</p>
            </contents>
        "#;

        let (body, metadata) = parse_post(content)?;
        assert!(metadata.tags.is_empty());
        assert_eq!(body, "<p>Content</p>");
        Ok(())
    }

    #[test]
    fn test_nested_content() -> anyhow::Result<()> {
        let content = r#"
            <metadata>
                <title>Title</title>
                <tags>
                    <tag>Test</tag>
                </tags>
            </metadata>
            <contents>
                <div>
                    <h1>Title</h1>
                    <p>Paragraph</p>
                </div>
            </contents>
        "#;

        let (body, metadata) = parse_post(content)?;
        assert_eq!(metadata.tags, vec!["Test"]);
        assert!(body.contains("Title"));
        assert!(body.contains("Paragraph"));
        Ok(())
    }

    #[test]
    fn test_html_with_attributes() -> anyhow::Result<()> {
        let content = r#"
            <metadata>
                <title>Title</title>
                <tags>
                    <tag class="category">Test</tag>
                </tags>
            </metadata>
            <contents>
                <div class="article">
                    <h1 id="title">Title</h1>
                    <p class="content">Paragraph</p>
                </div>
            </contents>
        "#;

        let (body, metadata) = parse_post(content)?;
        assert_eq!(metadata.tags, vec!["Test"]);
        assert!(body.contains("Title"));
        assert!(body.contains("Paragraph"));
        Ok(())
    }

        fn create_html_file(dir: &Path, name: &str, contents: &str) -> anyhow::Result<()> {
        let path = dir.join(name);
        let mut file = File::create(path)?;
        file.write_all(contents.as_bytes())?;
        Ok(())
    }

    #[test]
    fn test_empty_directory() -> anyhow::Result<()> {
        let temp_dir = TempDir::new()?;
        let posts = get_all_posts(temp_dir.path())?;
        assert_eq!(posts.len(), 0);
        Ok(())
    }

    #[test]
    fn test_html_with_metadata() -> anyhow::Result<()> {
        let temp_dir = TempDir::new()?;
        let content = r#"
            <metadata>
                <title>Title</title>
                <tags>
                    <tag class="category">Test</tag>
                </tags>
            </metadata>
            <contents>
                <div class="article">
                    <h1 id="title">Title</h1>
                    <p class="content">Paragraph</p>
                </div>
            </contents>
        "#;
        create_html_file(temp_dir.path(), "test.html", content)?;

        let posts = get_all_posts(temp_dir.path())?;
        assert_eq!(posts.len(), 1);
        
        let expected_body = r#"
                <div class="article">
                    <h1 id="title">Title</h1>
                    <p class="content">Paragraph</p>
                </div>
            "#.trim();
            
        assert_eq!(posts[0].body.trim(), expected_body);
        Ok(())
    }

    #[test]
    fn test_invalid_directory() {
        let non_existent = Path::new("/definitely/not/a/real/path");
        assert!(get_all_posts(non_existent).is_err());
    }

    #[test]
    fn test_empty_file() -> anyhow::Result<()> {
        let temp_dir = TempDir::new()?;
        create_html_file(temp_dir.path(), "empty.html", "")?;

        get_all_posts(temp_dir.path()).expect_err("Empty files should throw errors");

        Ok(())
    }
}
