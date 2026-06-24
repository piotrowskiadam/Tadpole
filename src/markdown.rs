use scraper::{Html, Selector, ElementRef};
use scraper::node::Node;
use ego_tree::NodeRef;
use crate::state::CrawlConfig;

/// Converts an HTML string into a clean Markdown string based on CrawlConfig rules.
pub fn html_to_markdown(html_str: &str, config: &CrawlConfig) -> String {
    let document = Html::parse_document(html_str);
    
    // 1. Parse ignored CSS selectors
    let mut ignored_selectors = Vec::new();
    if !config.md_ignored_selectors.trim().is_empty() {
        for sel_str in config.md_ignored_selectors.split(',') {
            let trimmed = sel_str.trim();
            if !trimmed.is_empty() {
                if let Ok(selector) = Selector::parse(trimmed) {
                    ignored_selectors.push(selector);
                }
            }
        }
    }

    // 2. Identify start node based on "only main content"
    let start_node = if config.md_only_main_content {
        let main_selectors = [
            "article",
            "main",
            "[role=\"main\"]",
            "#content",
            "#main",
            ".main",
            ".content",
        ];
        let mut found_ref = None;
        for sel in &main_selectors {
            if let Ok(selector) = Selector::parse(sel) {
                if let Some(el) = document.select(&selector).next() {
                    found_ref = Some(el);
                    break;
                }
            }
        }
        found_ref.map(|el| *el)
            .unwrap_or_else(|| *document.root_element())
    } else {
        *document.root_element()
    };

    let mut markdown = String::new();
    let mut list_index = None;
    
    walk_node(
        start_node,
        &mut markdown,
        &mut list_index,
        config,
        &ignored_selectors,
        0,
        false,
    );

    if config.md_clean_whitespace {
        markdown = clean_whitespace(&markdown);
    }

    markdown.trim().to_string()
}

fn walk_node<'a>(
    node: NodeRef<'a, Node>,
    markdown: &mut String,
    list_index: &mut Option<usize>,
    config: &CrawlConfig,
    ignored_selectors: &[Selector],
    indent_level: usize,
    in_pre: bool,
) {
    if let Some(element) = ElementRef::wrap(node) {
        let tag_name = element.value().name();
        
        // Skip script, style, navigation and head metadata elements completely
        let is_boilerplate = match tag_name {
            "script" | "style" | "head" | "iframe" | "noscript" | "svg" | "canvas" | "embed" | "object" | "param" | "form" | "select" | "option" => true,
            "header" | "footer" | "nav" | "aside" if config.md_only_main_content => true,
            _ => false,
        };
        if is_boilerplate {
            return;
        }

        // Check ignored selectors
        for selector in ignored_selectors {
            if selector.matches(&element) {
                return;
            }
        }

        match tag_name {
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                let level = tag_name[1..].parse::<usize>().unwrap_or(1);
                markdown.push_str("\n\n");
                markdown.push_str(&"#".repeat(level));
                markdown.push_str(" ");
                
                let mut inner = String::new();
                for child in node.children() {
                    walk_node(child, &mut inner, &mut None, config, ignored_selectors, indent_level, in_pre);
                }
                markdown.push_str(inner.trim());
                markdown.push_str("\n\n");
            }
            "p" => {
                markdown.push_str("\n\n");
                let mut inner = String::new();
                for child in node.children() {
                    walk_node(child, &mut inner, &mut None, config, ignored_selectors, indent_level, in_pre);
                }
                markdown.push_str(inner.trim());
                markdown.push_str("\n\n");
            }
            "br" => {
                markdown.push_str("\n");
            }
            "strong" | "b" => {
                markdown.push_str("**");
                for child in node.children() {
                    walk_node(child, markdown, &mut None, config, ignored_selectors, indent_level, in_pre);
                }
                markdown.push_str("**");
            }
            "em" | "i" => {
                markdown.push_str("*");
                for child in node.children() {
                    walk_node(child, markdown, &mut None, config, ignored_selectors, indent_level, in_pre);
                }
                markdown.push_str("*");
            }
            "code" => {
                if in_pre {
                    for child in node.children() {
                        walk_node(child, markdown, &mut None, config, ignored_selectors, indent_level, true);
                    }
                } else {
                    markdown.push_str("` ");
                    for child in node.children() {
                        walk_node(child, markdown, &mut None, config, ignored_selectors, indent_level, in_pre);
                    }
                    markdown.push_str(" `");
                }
            }
            "pre" => {
                markdown.push_str("\n\n```\n");
                let mut inner = String::new();
                for child in node.children() {
                    walk_node(child, &mut inner, &mut None, config, ignored_selectors, indent_level, true);
                }
                markdown.push_str(&inner);
                markdown.push_str("\n```\n\n");
            }
            "blockquote" => {
                markdown.push_str("\n\n");
                let mut inner = String::new();
                for child in node.children() {
                    walk_node(child, &mut inner, &mut None, config, ignored_selectors, indent_level, in_pre);
                }
                for line in inner.trim().lines() {
                    markdown.push_str("> ");
                    markdown.push_str(line);
                    markdown.push_str("\n");
                }
                markdown.push_str("\n");
            }
            "ul" => {
                markdown.push_str("\n\n");
                for child in node.children() {
                    walk_node(child, markdown, &mut None, config, ignored_selectors, indent_level + 1, in_pre);
                }
                markdown.push_str("\n\n");
            }
            "ol" => {
                markdown.push_str("\n\n");
                let mut idx = 1;
                for child in node.children() {
                    if let Some(child_el) = ElementRef::wrap(child) {
                        if child_el.value().name() == "li" {
                            walk_node(child, markdown, &mut Some(idx), config, ignored_selectors, indent_level + 1, in_pre);
                            idx += 1;
                        } else {
                            walk_node(child, markdown, &mut None, config, ignored_selectors, indent_level + 1, in_pre);
                        }
                    }
                }
                markdown.push_str("\n\n");
            }
            "li" => {
                markdown.push_str("\n");
                markdown.push_str(&"  ".repeat(indent_level.saturating_sub(1)));
                if let Some(idx) = list_index {
                    markdown.push_str(&format!("{}. ", idx));
                } else {
                    markdown.push_str("- ");
                }
                let mut inner = String::new();
                for child in node.children() {
                    walk_node(child, &mut inner, &mut None, config, ignored_selectors, indent_level, in_pre);
                }
                markdown.push_str(inner.trim());
                markdown.push_str("\n");
            }
            "a" => {
                let href = element.attr("href").unwrap_or("").trim();
                if config.md_keep_links && !href.is_empty() {
                    markdown.push_str("[");
                    let mut inner = String::new();
                    for child in node.children() {
                        walk_node(child, &mut inner, &mut None, config, ignored_selectors, indent_level, in_pre);
                    }
                    markdown.push_str(inner.trim());
                    markdown.push_str("](");
                    markdown.push_str(href);
                    markdown.push_str(")");
                } else {
                    for child in node.children() {
                        walk_node(child, markdown, &mut None, config, ignored_selectors, indent_level, in_pre);
                    }
                }
            }
            "img" => {
                if !config.md_ignore_images {
                    let src = element.attr("src").unwrap_or("").trim();
                    let alt = element.attr("alt").unwrap_or("").trim();
                    if !src.is_empty() {
                        markdown.push_str(" ![");
                        markdown.push_str(alt);
                        markdown.push_str("](");
                        markdown.push_str(src);
                        markdown.push_str(") ");
                    }
                }
            }
            "table" => {
                markdown.push_str("\n\n");
                let rows_sel = Selector::parse("tr").unwrap();
                let mut table_rows = Vec::new();
                for row_el in element.select(&rows_sel) {
                    let mut cells = Vec::new();
                    let cell_sel = Selector::parse("td, th").unwrap();
                    for cell_el in row_el.select(&cell_sel) {
                        let mut cell_text = String::new();
                        for child in cell_el.children() {
                            walk_node(child, &mut cell_text, &mut None, config, ignored_selectors, indent_level, in_pre);
                        }
                        cells.push(cell_text.trim().replace('\n', " ").replace('|', "\\|"));
                    }
                    if !cells.is_empty() {
                        table_rows.push(cells);
                    }
                }
                
                if !table_rows.is_empty() {
                    for (r_idx, cells) in table_rows.iter().enumerate() {
                        markdown.push_str("| ");
                        markdown.push_str(&cells.join(" | "));
                        markdown.push_str(" |\n");
                        if r_idx == 0 {
                            markdown.push_str("|");
                            for _ in cells {
                                markdown.push_str(" --- |");
                            }
                            markdown.push_str("\n");
                        }
                    }
                }
                markdown.push_str("\n\n");
            }
            _ => {
                for child in node.children() {
                    walk_node(child, markdown, &mut None, config, ignored_selectors, indent_level, in_pre);
                }
            }
        }
    } else if let Some(text_node) = node.value().as_text() {
        if in_pre {
            markdown.push_str(text_node);
        } else {
            let text = text_node.trim();
            if !text.is_empty() {
                let mut prefix_space = false;
                if text_node.starts_with(char::is_whitespace) && !markdown.is_empty() && !markdown.ends_with(char::is_whitespace) {
                    prefix_space = true;
                }
                
                let mut suffix_space = false;
                if text_node.ends_with(char::is_whitespace) {
                    suffix_space = true;
                }
                
                if prefix_space {
                    markdown.push_str(" ");
                }
                markdown.push_str(&text.split_whitespace().collect::<Vec<_>>().join(" "));
                if suffix_space {
                    markdown.push_str(" ");
                }
            }
        }
    }
}

fn clean_whitespace(input: &str) -> String {
    let mut cleaned = String::new();
    let mut consecutive_newlines = 0;
    
    for line in input.lines() {
        let line_val = line.trim_end();
        if line_val.trim_start().is_empty() {
            consecutive_newlines += 1;
        } else {
            if consecutive_newlines > 0 {
                cleaned.push_str("\n\n");
                consecutive_newlines = 0;
            } else if !cleaned.is_empty() && !cleaned.ends_with('\n') {
                cleaned.push_str("\n");
            }
            cleaned.push_str(line_val);
        }
    }
    
    cleaned
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_html_to_markdown_basic() {
        let html = r#"
            <!DOCTYPE html>
            <html>
            <head><title>Test Page</title></head>
            <body>
                <header>Header boilerplate</header>
                <nav>Navigation</nav>
                <main id="content">
                    <h1>Main Heading</h1>
                    <p>This is a <strong>paragraph</strong> with <em>italicized</em> text and a <a href="https://example.com">link</a>.</p>
                    <ul>
                        <li>Item 1</li>
                        <li>Item 2</li>
                    </ul>
                    <ol>
                        <li>First</li>
                        <li>Second</li>
                    </ol>
                    <pre><code>fn main() {
    println!("Hello World");
}</code></pre>
                    <blockquote>
                        Quote line 1<br>
                        Quote line 2
                    </blockquote>
                    <img src="img.jpg" alt="Test Image">
                </main>
                <footer>Footer boilerplate</footer>
            </body>
            </html>
        "#;

        let mut config = CrawlConfig::default();
        config.md_only_main_content = true;
        config.md_keep_links = true;
        config.md_ignore_images = false;
        config.md_clean_whitespace = true;

        let md = html_to_markdown(html, &config);

        assert!(md.contains("# Main Heading"));
        assert!(md.contains("This is a **paragraph** with *italicized* text"));
        assert!(md.contains("[link](https://example.com)"));
        assert!(md.contains("- Item 1"));
        assert!(md.contains("1. First"));
        assert!(md.contains("```"));
        assert!(md.contains("println!(\"Hello World\")"));
        assert!(md.contains("> Quote line 1"));
        assert!(md.contains("> Quote line 2"));
        assert!(md.contains("![Test Image](img.jpg)"));
        // Boilerplate should be skipped
        assert!(!md.contains("Header boilerplate"));
        assert!(!md.contains("Footer boilerplate"));
        assert!(!md.contains("Navigation"));
    }

    #[test]
    fn test_html_to_markdown_exclusions() {
        let html = r#"
            <body>
                <div class="ads">Ad banner</div>
                <div class="main-content">
                    <p>Main text</p>
                    <div id="comments">User comments here</div>
                </div>
            </body>
        "#;

        let mut config = CrawlConfig::default();
        config.md_only_main_content = false;
        config.md_ignored_selectors = ".ads, #comments".to_string();

        let md = html_to_markdown(html, &config);
        assert!(md.contains("Main text"));
        assert!(!md.contains("Ad banner"));
        assert!(!md.contains("User comments here"));
    }
}
