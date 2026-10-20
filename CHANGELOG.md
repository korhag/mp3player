# Changelog

All notable changes to Soundkeep are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.9] - 2026-10-20

### Fixed

- Dragging a favorite keeps the list where you scrolled. Drop a show to reorder it, group it, or take it out of a group. A short click still opens the show.
- While a show is held, the bottom of the favorites list says "Drop here to remove from group."

## [0.1.8] - 2026-10-20

### Fixed

- Dropping a favorite onto another show in the same group moves it to that show. A label with the show name follows the pointer while you hold it.

## [0.1.7] - 2026-10-20

### Fixed

- Click or double-click a favorite show to open it. Hold a show and drop it on the top or bottom of a row to reorder, or on the middle of another show to group them.

## [0.1.6] - 2026-10-20

### Added

- The app reopens on the last tab, and on the podcast that was open when it closed.
- Header logo with the Soundkeep name, recolored so it stays readable on the current theme.

### Changed

- Renamed the app from MP3Player to Soundkeep, including the binary, installers, and data folder. An existing library moves to the new folder on first launch.
- Enlarged the play and pause button and placed it on a faint chip.

### Fixed

- Holding a favorite and dropping it on another show groups them. Dropping it on the line between rows reorders it. A short click still opens the show.
- Dragging a favorite show or group keeps its drop target when the list redraws.
- Reinstalling Soundkeep closes a running copy first, so the new program file can replace the old one.
- Windows title bar and taskbar show the app icon.
