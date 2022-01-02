# Changelog

Updates for all releases of the various tools

# 0.2.0 - Jan 02, 2022

- Updated to include auditing capabilities to see when a given module turned on ESM in some way.
- Refined stats table to have a partion key of the month and year and a sort key of timestamp.
  - this will allow queries to be made against the table and only have ~30 items read at max per partition.

# 0.1.1 - Dec 30, 2021

- fixed bug on macOS which was causing DNS issues due to too many concurrent requests.

## 0.1.0 - Dec 29, 2021

- initial impmlementation
