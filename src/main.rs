// This example Compute@Edge app modifies HTML returned from https://example.com at the edge
use fastly::http::{header, StatusCode};
use fastly::{Error, Request, Response};
use lol_html::html_content::ContentType;
use lol_html::{element, rewrite_str, text, RewriteStrSettings};

// Load files into the constants
const FONT_LINKS: &str = include_str!("font.html");
const STYLE: &str = include_str!("style.css");

#[fastly::main]
fn main(mut req: Request) -> Result<Response, Error> {
    // Tell robots not to index
    if req.get_url().path() == "/robots.txt" {
        return Ok(Response::from_status(StatusCode::OK).with_body("User-agent: *\nDisallow: /"));
    }
    // Add the host header so that you don't need to specify it in a request when testing locally
    req.set_header(header::HOST, "example.com");
    // Request an uncompressed response
    req.remove_header(header::ACCEPT_ENCODING);
    let beresp = req.send("example_com")?;

    if beresp.get_status() == StatusCode::OK {
        println!("Returning a modified version of https://example.com");
        return Ok(rewrite_html(beresp));
    }
    // For responses other than 200, just return the origin response as is
    Ok(beresp)
}

fn rewrite_html(beresp: Response) -> Response {
    let resp = beresp.clone_without_body();
    let element_content_handlers = vec![
        // Insert Google Fonts link tags
        element!("meta[name]", |e| {
            e.after(FONT_LINKS, ContentType::Html);
            Ok(())
        }),
        // Replace CSS in the style tags
        element!("style", |e| {
            e.set_inner_content(STYLE, ContentType::Text);
            Ok(())
        }),
        // Modify inner contents of h1 tags - enclose each word with span tags.
        // "<h1>Example Domain</h1>" -> "<h1><span>Example</span><span>Domain</span></h1>"
        text!("h1", |t| {
            if !t.last_in_text_node() {
                let tagged_t = t
                    .as_str()
                    .split(" ")
                    .map(|w| format!("<span>{}</span>", w))
                    .fold(String::new(), |mut acc, cur| {
                        acc.push_str(cur.as_str());
                        acc
                    });
                t.replace(&tagged_t, ContentType::Html);
            }
            Ok(())
        }),
    ];

    let html = rewrite_str(
        &beresp.into_body_str(),
        RewriteStrSettings {
            element_content_handlers,
            ..RewriteStrSettings::default()
        },
    )
    .unwrap();

    resp.with_body(html)
}
