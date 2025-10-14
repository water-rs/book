# WaterUI Tutorial Book

[![License: CC BY 4.0](https://img.shields.io/badge/License-CC%20BY%204.0-lightgrey.svg)](http://creativecommons.org/licenses/by/4.0/)

The complete guide to building cross-platform applications with WaterUI! This book takes you from beginner to advanced WaterUI developer, teaching you how to build sophisticated applications that run on desktop, web, mobile, and embedded platforms.

## 📖 Read the Book

The book is available online at: **[book.waterui.dev](https://book.waterui.dev)**

Or build it locally:

```bash
# Install mdBook if you haven't already
cargo install mdbook

# Build and serve the book
mdbook serve --open
```

## 🚀 What You'll Learn

- **Getting Started**: Set up your development environment and create your first WaterUI app
- **Core Concepts**: Master views, reactive state management, and the environment system
- **Basic Components**: Work with layouts, text, forms, media, and navigation
- **Advanced Topics**: Animations, suspense, error handling, plugins, accessibility, i18n, canvas, and shaders
- **Internals**: Deep dive into WaterUI's rendering engine, hot reload, and layout system

## 📋 Prerequisites

- Basic Rust knowledge (ownership, borrowing, traits, generics)
- Programming experience
- Command line familiarity

New to Rust? Start with [The Rust Programming Language](https://doc.rust-lang.org/book/) first.

## 🛠️ Building the Book

This book is built with [mdBook](https://rust-lang.github.io/mdBook/).

```bash
# Clone the repository
git clone https://github.com/water-rs/book.git
cd book

# Install dependencies
cargo install mdbook

# Build the book
mdbook build

# Serve locally with live reload
mdbook serve
```

The built book will be in the `book/` directory.

## 🤝 Contributing

We welcome contributions! Found a typo, unclear explanation, or want to add content?

1. Fork this repository
2. Create a new branch (`git checkout -b fix/typo-in-chapter-3`)
3. Make your changes
4. Commit your changes (`git commit -am 'Fix typo in chapter 3'`)
5. Push to the branch (`git push origin fix/typo-in-chapter-3`)
6. Open a Pull Request

### Guidelines

- Keep explanations clear and concise
- Include code examples where appropriate
- Test code examples to ensure they work
- Follow the existing style and formatting
- Update the table of contents in `SUMMARY.md` if adding new chapters

## 📄 License

This work is licensed under a [Creative Commons Attribution 4.0 International License](http://creativecommons.org/licenses/by/4.0/).

Copyright (c) 2025 Lexo Liu

You are free to share and adapt this material for any purpose, even commercially, as long as you give appropriate credit.

## 🔗 Links

- [WaterUI Repository](https://github.com/water-rs/waterui)
- [WaterUI Website](https://waterui.dev)
- [Roadmap](https://waterui.dev/roadmap)
- [Community](https://github.com/water-rs/waterui/discussions)

## 📞 Support

- **Issues**: [GitHub Issues](https://github.com/water-rs/book/issues)
- **Discussions**: [GitHub Discussions](https://github.com/water-rs/waterui/discussions)

---

Made with ❤️ by the WaterUI community
