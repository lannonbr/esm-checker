# ESM Checker

A Rust tool to check what popular node projects are using [ESM](https://nodejs.org/api/esm.html) for package their code.

## Requisites

If you want to test this locally you'll need the following:

- Node & the npm cli. (Any LTS should work)
- Rust (Latest stable should be good)

## Files & Directories

- `registry.txt`: a comma separated list of all packages on npm as of a snapshot of the entire public npm registry as of December 23, 2021.
- `node/package.json`: a collection of the top 250 most depended on packages as based on this Gist: [01.most-dependent-on.md](https://gist.github.com/anvaka/8e8fa57c7ee1350e3491#file-01-most-dependent-upon-md). From here I can download all of their package.json files and then run them through various tools in this repository.
- `src/`: the source for the rust tooling to examine various node projects.

## Running

First, make sure to run `npm install` within the `node` folder to get all of the various modules downloaded.

Then you can run any of the binaries using the `--bin` flag on `cargo run` and then list the name of the file you want to run.

Ex: `cargo run --bin aggregate-package-prefixes`
