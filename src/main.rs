use std::io::prelude::*;
use std::io::Error;
use std::fs::{self, File};
use std::path::PathBuf;
use std::ffi::OsStr;
use std::collections::HashMap;
use tera::{Tera, Context};
use pulldown_cmark::{Parser, Options, html};
use walkdir::{DirEntry, WalkDir};
use heck::KebabCase;

mod front_matter;

#[derive(Default)]
pub struct Site {
    pub site_name: String,
    base_dir: String,
    // base_url: String,
    posts: HashMap<PathBuf, String>,
    pages: HashMap<PathBuf, String>,
}

impl Site {
    pub fn new(name: &str) -> Result<Self, Error> {
        let base_dir = Self::create_new_project(name).unwrap();
        
         Ok(Site {
            site_name: name.to_string(),
            // base_url: String::from("http:://example.com"),
            base_dir,
            pages: HashMap::new(),
            posts: HashMap::new(),
        })
    }

    fn create_new_project(site_name: &str) -> Result<String, Error> {
        let base_dir = site_name.to_kebab_case();

        println!("New project in {}", &base_dir);

        fs::create_dir_all(&base_dir)?; 
        fs::create_dir_all(format!("{}/_includes", &base_dir))?; 
        fs::create_dir_all(format!("{}/_layouts", &base_dir))?; 
        fs::create_dir_all(format!("{}/posts", &base_dir))?; 
        fs::create_dir_all(format!("{}/pages", &base_dir))?; 
        fs::create_dir_all(format!("{}/assets", &base_dir))?; 
        fs::create_dir_all(format!("{}/assets/img", &base_dir))?; 
        fs::create_dir_all(format!("{}/assets/css", &base_dir))?; 
        fs::create_dir_all(format!("{}/assets/js", &base_dir))?; 

        let default_content = "<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n<title>{{ site_name }}</title>\n</head>\n<body>{{ content }}</body>\n</html>";
        let mut default_layout = File::create(format!("{}/_layouts/default.html", &base_dir))?;
        default_layout.write_all(default_content.as_bytes())?;

        let post_content = "---\nlayout: page\n---\n{{ content }}";
        let mut post_layout = File::create(format!("{}/_layouts/post.html", &base_dir))?;
        post_layout.write_all(post_content.as_bytes())?;

        let page_content = "---\nlayout: default\n---\n{{ content }}";
        let mut page_layout = File::create(format!("{}/_layouts/page.html", &base_dir))?;
        page_layout.write_all(page_content.as_bytes())?;

        let config_content = "baseURL = \"http://example.org/\"\ntitle = \"My New Hugo Site\"";
        let mut config_layout = File::create(format!("{}/config.toml", &base_dir))?;
        config_layout.write_all(config_content.as_bytes())?;

        Ok(base_dir)
    }

    pub fn build(&mut self) {
        self.create_build_folder().unwrap();
        self.render_posts();
        self.render_pages().unwrap();
    } 

    fn create_build_folder(&self) -> Result<(), Error> {
        let build_dir = format!("{}/public", self.base_dir);
        println!("Building project in {}", build_dir);

        fs::create_dir_all(&build_dir)?;
        fs::create_dir_all(format!("{}/posts", &build_dir))?; 
        fs::create_dir_all(format!("{}/assets", &build_dir))?; 
        fs::create_dir_all(format!("{}/assets/img", &build_dir))?; 
        fs::create_dir_all(format!("{}/assets/css", &build_dir))?; 
        fs::create_dir_all(format!("{}/assets/js", &build_dir))?; 

        Ok(())
    }
    
    fn markdown_to_html(markdown_input: &str) -> String {
        let mut options = Options::empty();
        options.insert(Options::ENABLE_STRIKETHROUGH);
        let parser = Parser::new_ext(markdown_input, options);

        let mut html_output = String::new();
        html::push_html(&mut html_output, parser);

        html_output
    }

    fn render_post(&self, path: &PathBuf) -> Result<String, Error> {
        let input = fs::read_to_string(&path).unwrap();

        let (matter, content) = front_matter::parse_and_find_content(input.as_str()).unwrap();
        let markdown_output = Self::markdown_to_html(content);

        let matter = matter.unwrap();
        let layout = "post";
        let title = matter["title"].as_str().unwrap();

        let mut context = Context::new();
        context.insert("title", title);
        context.insert("site_name", &self.site_name);

        let post_output = Tera::one_off(markdown_output.as_str(), &context, false).expect("Failed to render template");

        context.insert("content", &post_output);

        let render_layouts = Self::render_layouts(&self, layout, &mut context).unwrap();

        Ok(render_layouts.to_owned())
    }

    fn render_page(&self, path: &PathBuf) -> Result<String, Error> {
        let input = fs::read_to_string(&path).unwrap();

        let (matter, content) = front_matter::parse_and_find_content(input.as_str()).unwrap();
        let matter = matter.unwrap();
        println!("{}", content);

        let layout = matter["layout"].as_str().unwrap();
        let title = matter["title"].as_str().unwrap();

        let mut context = Context::new();
        context.insert("title", title);
        context.insert("site_name", &self.site_name);

        let page_output = Tera::one_off(content, &context, false).expect("Failed to render template");

        context.insert("content", &page_output);

        let render_layouts = Self::render_layouts(&self, layout, &mut context).unwrap();

        Ok(render_layouts.to_owned())
    }

    fn render_posts(&mut self) -> Result<(), Error> {
        let input_dir = format!("{}/posts", self.base_dir);
        let output_dir = format!("{}/public/posts", self.base_dir);

        let mut posts: HashMap<PathBuf, String> = HashMap::new();

        for entry in WalkDir::new(input_dir.as_str()).into_iter().filter_map(|e| e.ok()) {
            if entry.metadata().unwrap().is_file() {
                let post_path = entry.path().to_path_buf();

                match Self::get_file_name(&entry) {
                    Some(file_name) => {
                        let output_path = format!("{}/{}", &output_dir, &file_name);
                        println!("{}", output_path);

                        let post_output = Self::render_post(&self, &post_path).unwrap();
                        
                        let mut post = File::create(&output_path)?;
                        post.write_all(post_output.as_bytes())?;

                        posts.insert(post_path, String::from(&output_path)); 
                    },
                    None => println!("Oh no."),
                }
            }

        }

        self.posts = posts;

        Ok(())
    }

    fn render_pages(&mut self) -> Result<(), Error> {
        let input_dir = format!("{}/pages", self.base_dir);
        let output_dir = format!("{}/public", self.base_dir);

        let mut pages: HashMap<PathBuf, String> = HashMap::new();

        for entry in WalkDir::new(input_dir.as_str()).into_iter().filter_map(|e| e.ok()) {
            if entry.metadata().unwrap().is_file() {
                let page_path = entry.path().to_path_buf();

                match Self::get_file_name(&entry) {
                    Some(file_name) => {
                        let output_path = format!("{}/{}", &output_dir, &file_name);
                        println!("{}", output_path);

                        let page_output = Self::render_page(&self, &page_path).unwrap();
                        
                        let mut page_write = File::create(&output_path)?;
                        page_write.write_all(page_output.as_bytes())?;

                        pages.insert(page_path, String::from(&output_path)); 
                    },
                    None => println!("Oh no."),
                }
            }

        }

        self.pages = pages;

        Ok(())
    }

    fn get_file_name(entry: &DirEntry) -> Option<&str> {
        entry.path().file_name().and_then(OsStr::to_str)
    }

    fn render_layouts(&self, variant: &str, mut context: &mut Context) -> Result<String, Error> {
        let layout_path = format!("{}/_layouts/{}.html", self.base_dir, variant);
        let input = fs::read_to_string(&layout_path.as_str()).unwrap();

        println!("{} {}", layout_path, input);

        let (matter, content) = front_matter::parse_and_find_content(input.as_str()).unwrap();
        match matter {
            Some(matter) => {
                let nested_layout = matter["layout"].as_str().unwrap();
                let layout_output = Tera::one_off(content, &context, false).expect("Failed to render template");
                context.insert("content", &layout_output);
                println!("Calling recursion: {} {}", variant, nested_layout);
                let layout_output = Self::render_layouts(&self, nested_layout, &mut context).unwrap();
                return Ok(layout_output);
            },
            None => println!("Not nested"),
        }

        let layout_output = Tera::one_off(content, &context, false).expect("Failed to render template");


        Ok(layout_output)
    }
}

fn main() {
    let mut site = Site::new("Test").unwrap();
    site.build();
}
