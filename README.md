# Rust Static Site Generator

A minimal static site generator written in Rust that converts Markdown into a static site with section navigation, breadcrumbs, blog cards, pagination, and blog tags.

## What it does

- Reads Markdown files from `content/`
- Converts them into HTML pages
- Writes generated files to `public/` while preserving folder structure
- Copies non-Markdown files, like images and PDFs, from `content/` to `public/`
- Supports a multi-template layout system
- Generates blog index cards and pagination from `content/blog/*.md`
- Supports blog post title/subheading rendering from Markdown headings
- Supports blog post tags and tag archive pages

## How the templates work

This project uses three template files from `templates/`:

### `templates/page.html`
The shared outer shell for every page.

It is responsible for:
- the document structure
- the `<head>` section
- the site-wide CSS
- the navigation area
- the breadcrumbs area
- the page content placeholder

Placeholders used in this template:
- `{{title}}` - page title
- `{{robots_meta}}` - optional robots meta tag
- `{{nav}}` - top navigation HTML
- `{{breadcrumbs}}` - breadcrumb HTML
- `{{content}}` - page body HTML

### `templates/blog_index.html`
The blog landing/listing template.

It is responsible for the internal blog page layout, including:
- blog intro text
- blog post cards
- pagination controls

Placeholders used in this template:
- `{{blog_intro}}`
- `{{blog_posts}}`
- `{{pagination}}`

### `templates/blog_post.html`
The individual blog post template.

It is responsible for how a single blog post is structured on the page.

Placeholders used in this template:
- `{{post_title}}`
- `{{post_subheading_block}}`
- `{{post_body}}`

## Content structure

### Sections
Directories in `content/` become sections. The top-level navigation shows each section link, and the current section is highlighted.

### Root-level pages
Files directly in `content/` (like `index.md`) do not become separate sections.

### Breadcrumbs
Each page shows a breadcrumb trail, for example:
- Root page: `Home`
- Section page: `Home > Blog`
- Nested page: `Home > Blog > First Post`

## Example structure

```
content/
├── index.md              → public/index.html
├── about/
│   └── index.md          → public/about/index.html
├── assets/
│   └── cv.pdf            → public/assets/cv.pdf
└── blog/
    ├── index.md          → public/blog/index.html
    └── first-post.md     → public/blog/first-post.html

templates/
├── page.html
├── blog_index.html
└── blog_post.html
```

## Blog post format

For blog posts in `content/blog/*.md` except `index.md`, use this pattern:

```md
# Post Title
## Post Subheading
tags: rust, web, static site

Post body starts here...
```

- `#` becomes the post title
- `##` becomes the post subheading
- `tags:` declares a comma-separated list of tags
- The remaining Markdown becomes the post body
- The blog index renders each post as a card linking to the post page
- The blog index also shows tag filter links, and each tag gets its own archive page under `blog/tags/<tag>/`

## Blog pagination

Set `BLOG_POSTS_PER_PAGE` to control how many posts appear on each page:

```bash
BLOG_POSTS_PER_PAGE=5 cargo run
```

## Run locally

```bash
cargo run
```

### Optional environment variables

- `BASE_URL` - set this when the site is served from a subpath, such as a GitHub Pages project site
  - Example: `BASE_URL=/rust-personal-site cargo run`
- `NOINDEX=1` - adds a robots noindex meta tag and generates a blocking `robots.txt`

## Split repo setup

If you want to keep the generator public but store content privately, use two repos:

- Public repo: generator code
- Private repo: Markdown content, assets, and templates

### 1. Create the private content repo

Create a private repo named `rust-personal-site-content` with this structure:

```text
content/
templates/
  page.html
  blog_index.html
  blog_post.html
```

Put your posts, pages, images, PDFs, and template files there.

### 2. Add the private repo token

Create a fine-grained GitHub token with access to the private content repo and `Contents: Read` permission.

Then add it to the public repo as a secret:
- Name: `PRIVATE_REPO_TOKEN`
- Value: the token

### 3. Set up GitHub Actions

The workflow in the public repo:
- checks out the public generator code
- checks out the private content repo
- copies the private `content/` and `templates/` folders into the build workspace
- runs the generator
- deploys the generated `public/` output to GitHub Pages

### 4. Work on the site

Typical workflow:
1. Update content or templates in the private repo
2. Push changes to the private repo
3. Push generator changes to the public repo when needed
4. GitHub Actions rebuilds and deploys the site

### 5. Local development

For local work, clone both repos and copy the private `content/` and `templates/` into the public repo workspace before running `cargo run`.

## GitHub Pages

The site is configured for GitHub Pages project hosting.

- The workflow sets `BASE_URL` automatically from the repo name
- The workflow currently sets `NOINDEX=1` for staging-style deploys
- If you want the site publicly indexed later, remove `NOINDEX=1` from the workflow

## Notes

- `public/` is generated output and should not be committed
- `content/` and `templates/` are ignored in the public repo when using the split-repo setup
- If a template file is missing, the build fails immediately
