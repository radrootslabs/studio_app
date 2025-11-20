# Contributing

Rad Roots is an open-source project, and we welcome all contributions — including code improvements, bug fixes, translations, new features, and bug reports.

## Translations

If you would like to contribute by translating the app into another language, please visit our [localisation repository](https://github.com/radrootslabs/packages-locales). To contribute translations:

1. *Fork the repository and add your language files following the existing structure.*

2. *Submit a Pull Request with your changes.*


If the language you would like to add translations for is not yet set up in the repository let us know by [opening an issue](https://github.com/radrootslabs/packages-locales/issues), or [email us](mailto:support@radroots.dev) and we will assist you in adding the required files.

## Development Environment

Ensure the required system dependencies are installed:

*Rust*
```bash
$ cargo --version
> cargo 1.88.0 (873a06493 2025-05-10)
```

*NodeJS*
```bash
$ node --version
> v20.18.0
```

*Yarn*
```bash
$ yarn --version
> 1.22.22
```

## Building

This app is implemented as a progressive web application using [SvelteKit](https://svelte.dev/), and maintained as a monorepository using [Turbo](https://turborepo.com/) with individual packages tracked as Git submodules. 

To begin, first clone the repository and set up your local working copy:

```bash
mkdir pwa && cd pwa

git clone https://github.com/radrootslabs/pwa .

git remote rename origin upstream

git remote add origin https://github.com/<YOUR-USERNAME>/pwa.git

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

Install the application dependencies:
```bash
yarn
```


Configure local environment variables:
```bash
echo 'PUBLIC_NOSTR_RELAY_DEFAULTS=ws://localhost:8080,ws://localhost:8081
PUBLIC_RADROOTS_RELAY_URL=ws://localhost:8082
PUBLIC_RADROOTS_URL=https://radroots.org
VITE_PUBLIC_KEYVAL_NAME=rad-roots-pwa-dev-v1
VITE_PUBLIC_NDK_CACHE_NAME=rad-roots-pwa-dev-v1
VITE_PUBLIC_NDK_CLIENT_NAME=rad roots' > app/.env.development
```

Build the application:
```bash
yarn build
```

Run the application in development mode:
```bash
yarn dev
```

## Contributing

1. Create a feature branch
2. Make your changes
3. Submit a Pull Request
4. Wait for review and address feedback

## Additional Resources

- [Rust Documentation](https://www.rust-lang.org/tools/install)
- [NodeJS Documentation](https://github.com/nvm-sh/nvm?tab=readme-ov-file#installing-and-updating)
- [Yarn Documentation](https://classic.yarnpkg.com/en/docs)
- [SvelteKit Documentation](https://svelte.dev/docs/kit/introduction)
- [Turbo Documentation](https://turborepo.com/docs)

## License

Refer to the LICENSE file in the repository for terms of use and distribution.