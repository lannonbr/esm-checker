# ESM Checker

A Rust tool to check what popular node projects are using [ESM](https://nodejs.org/api/esm.html) for package their code.

## Requisites

If you want to test this locally you'll need the following:

- Rust (Latest stable should be good)

## Files & Directories

- `registry.txt`: a comma separated list of all packages on npm as of a snapshot of the entire public npm registry as of December 23, 2021.
- `packages.txt`: a collection of the top 250 most depended on packages as based on this Gist: [01.most-dependent-on.md](https://gist.github.com/anvaka/8e8fa57c7ee1350e3491#file-01-most-dependent-upon-md) and all of their dependencies (which equates to around 2,000 packages right now).
- `src/`: the source for the rust tooling to examine the node packages.
- `cdk/`: The source for the AWS CDK Stack for provisioning AWS resources to store historical data for this project

## Running

You can run any of the binaries using the `--bin` flag on `cargo run` and then list the name of the file you want to run.

Ex: `cargo run --bin aggregate-package-prefixes`

## Website

If you would like to see the data collected from this project visualized, visit https://esm-checker.netlify.app.
