# Rust Static Site Generator

A minimal static site generator written in Rust that converts Markdown to HTML with section-based navigation and breadcrumbs.

## What it does

- Reads Markdown files from `content/`
- Converts them to HTML pages
- Writes generated files to `public/` preserving folder structure
- Copies non-Markdown files (like PDFs/images) from `content/` to `public/`
- Uses `templates/page.html` as the page template
- **Automatically generates section-based navigation**
- **Adds breadcrumbs** showing page hierarchy

## Structure & Navigation

### Sections
Directories in `content/` become sections. The top-level nav shows all sections with links to each section's index page. The current section is highlighted.

### Root-level pages
Files directly in `content/` (like `index.md`, `about.md`) don't appear as separate sections—they're just part of the home nav.

### Breadcrumbs
Each page shows:
- **Root pages** (e.g., `about.md` → `about.html`): `Home`
- **Section pages** (e.g., `blog/index.md`): `Home > Blog`
- **Nested pages** (e.g., `blog/first-post.md`): `Home > Blog > First Post`

## Example structure

```
content/
├── index.md              → public/index.html (nav: Home | Blog)
├── about/
│   └── index.md          → public/about/index.html (nav: Home | About* | Blog)
├── assets/
│   └── cv.pdf            → public/assets/cv.pdf
└── blog/
    ├── index.md          → public/blog/index.html (nav: Home | About | Blog*)
    └── first-post.md     → public/blog/first-post.html (nav: Home | About | Blog*)

*  Current section is bold
```

## Template placeholders

Your custom template can use:
- `{{title}}` - Page title (from first `#` heading or filename)
- `{{content}}` - Rendered HTML from Markdown
- `{{nav}}` - Section navigation (`<nav>` with links)
- `{{breadcrumbs}}` - Breadcrumb trail (`<div class="breadcrumbs">`)

See `templates/page.html` for the default template with styling.

## Run

```bash
cargo run
```

Generated pages appear in `public/`.

## Base URL for deployments

Set `BASE_URL` when your site is served from a subpath (like GitHub Pages project sites).

- Personal site (`username.github.io`): leave `BASE_URL` empty
- Project site (`username.github.io/repo-name`): set `BASE_URL=/repo-name`

Examples:

```bash
cargo run
BASE_URL=/rust-personal-site cargo run
```

The included GitHub Actions workflow already sets `BASE_URL` to the repository name automatically.

## Search indexing control

Set `NOINDEX=1` to add `<meta name="robots" content="noindex,nofollow,noarchive">` to every page and generate a blocking `robots.txt`.

Examples:

```bash
NOINDEX=1 cargo run
BASE_URL=/rust-personal-site NOINDEX=1 cargo run
```

The GitHub Actions workflow currently deploys with `NOINDEX=1` for staging privacy-by-obscurity. Remove that env var when you want pages indexable.

