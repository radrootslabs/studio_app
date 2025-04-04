# Contributing

Tangle is an open-source project, and we welcome all contributions — including code improvements, bug fixes, translations, new features, and bug reports.

## Translations

If you would like to contribute by translating the app into another language, please visit our [localisation repository](https://github.com/72-61-64-72-6f-6f-74-73/packages-locales). To contribute translations:

1. *Fork the repository and add your language files following the existing structure.*

2. *Submit a Pull Request with your changes.*


If the language you would like to add translations for is not yet set up in the repository let us know by [opening an issue](https://github.com/72-61-64-72-6f-6f-74-73/packages-locales/issues), or [email us](mailto:support@radroots.dev) and we will assist you in adding the required files.

## Development Environment

Ensure the required system dependencies are installed:

*Rust*
```bash
$ cargo --version
> cargo 1.81.0 (2dbb1af80 2024-08-20)
```

*NodeJS*
```bash
$ node --version
> v20.18.0
```

*Pnpm*
```bash
$ pnpm --version
> 9.12.1
```

## Building

Tangle is built as a static application with [SvelteKit](https://svelte.dev/) and bundled into native binaries for iOS and Android with [Tauri](https://v2.tauri.app/).

The project is structured as a monorepository with individual packages managed as Git submodules. 

To begin, first clone the repository and set up your local working copy:

```bash
mkdir tangle && cd tangle

git clone https://github.com/72-61-64-72-6f-6f-74-73/tangle .

git remote rename origin upstream

git remote add origin https://github.com/<YOUR-USERNAME>/tangle.git

git push -u origin master
```

Initialize and update Git submodules:
```bash
git submodule update --init --recursive
```

Checkout Git submodules branches:
```bash
git submodule foreach 'git checkout $(git config -f $toplevel/.gitmodules submodule.$name.branch)'
```

Install the application dependencies (Rust):
```bash
cargo check
```

Install the application dependencies (JavaScript):
```bash
pnpm install
```


Configure local environment variables:
```bash
echo 'PUBLIC_NOSTR_RELAY_DEFAULTS=ws://localhost:8080,ws://localhost:8081
PUBLIC_RADROOTS_RELAY_URL=ws://localhost:8082
PUBLIC_RADROOTS_URL=https://radroots.market
VITE_PUBLIC_IDB_NAME=tangle-dev-v1
VITE_PUBLIC_NDK_CACHE_NAME=tangle-dev-v1
VITE_PUBLIC_NDK_CLIENT_NAME=tangle-dev-v1' > app/.env.development
```

Build the web application:
```bash
pnpm run build
```

Run the native application in development mode:
```bash
pnpm run tauri dev
```

## Contributing

1. Create a feature branch
2. Make your changes
3. Submit a Pull Request
4. Wait for review and address feedback

## Additional Resources

- [Rust Documentation](https://www.rust-lang.org/tools/install)
- [NodeJS Documentation](https://github.com/nvm-sh/nvm?tab=readme-ov-file#installing-and-updating)
- [Pnpm Documentation](https://pnpm.io/installation)
- [SvelteKit Documentation](https://svelte.dev/docs/kit/introduction)
- [Tauri Documentation](https://v2.tauri.app/start/prerequisites/)
- [XCode Documentation](https://developer.apple.com/documentation/safari-developer-tools/installing-xcode-and-simulators)
- [Android Studio Documentation](https://developer.android.com/studio/install)

## License

Refer to the LICENSE file in the repository for terms of use and distribution.