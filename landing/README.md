# GigaWhisper Landing Page

A simple, static landing page for GigaWhisper.

## Structure

```
landing/
├── index.html      # Main landing page
├── assets/
│   ├── logo.svg    # Logo (microphone icon)
│   └── favicon.svg # Browser favicon
└── README.md       # This file
```

## Deployment

This is a static site that can be deployed anywhere:

### GitHub Pages

1. Go to repository Settings > Pages
2. Set source to "Deploy from a branch"
3. Select the branch and `/landing` folder
4. Save and wait for deployment

### Netlify / Vercel

1. Connect your GitHub repository
2. Set build output directory to `landing`
3. No build command needed (static files)

### Manual

Simply upload the `landing` folder contents to any web server.

## Development

Open `index.html` directly in a browser to preview. Uses Tailwind CSS via CDN for styling.

## Customization

- **Colors**: Edit the Tailwind config in `<script>` tag or the `.gradient-bg` class
- **Content**: Edit the HTML directly
- **Logo**: Replace `assets/logo.svg` with your own SVG

## Technologies

- HTML5
- Tailwind CSS (via CDN)
- Custom CSS animations
- SVG icons
