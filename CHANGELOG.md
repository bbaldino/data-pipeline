# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/bbaldino/data-pipeline/releases/tag/v0.1.0) - 2024-10-11

### Fixed

- fix tests

### Other

- add gh actions
- change crate name
- add attach_handler method to pipelinebuilder
- update readme with handler implementation examples
- don't make handlers stats producers, do it on the SomeDataHandler enum
- add README
- improve builder syntax
- implement special visitor logic for demuxer
- remove explicit references to 'packet'
- change crate name
- make nodes, etc. generic over the data they handle
- add demuxer
- code cleanup, tweaks
- initial commit
