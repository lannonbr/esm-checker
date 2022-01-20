# Changelog

Updates for all releases of the various tools

## 0.3.0 - Jan 19, 2022

- Added discord webhook script to message a given week's stats update.

## 0.2.2 - Jan 15, 2022

- Updated dependencies
  - migrated StructOpt to Clap

## 0.2.1 - Jan 04, 2022

- Add uuid to audit entry sort keys

## 0.2.0 - Jan 02, 2022

- Updated to include auditing capabilities to see when a given module turned on ESM in some way.
- Refined stats table to have a partion key of the month and year and a sort key of timestamp.
  - this will allow queries to be made against the table and only have ~30 items read at max per partition.

## 0.1.1 - Dec 30, 2021

- fixed bug on macOS which was causing DNS issues due to too many concurrent requests.

## 0.1.0 - Dec 29, 2021

- initial impmlementation
