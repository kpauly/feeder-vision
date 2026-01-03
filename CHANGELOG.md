# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

## [1.3.0] - 2026-01-03

### Added
- Fluent-based localization with system auto-detect, language override, and native language names in the dropdown.
- UI translations for Dutch, English, French, German, Spanish, and Swedish plus translated species labels.
- Windows in-app updater workflow (download + hash/size validation + installer launch via FeedieUpdater).
- Recursive scan option with cached result re-open.
- Linux AppImage releases (x86_64 + aarch64) with AppImage model/resource fallbacks for APPDIR and /usr/share.
- Chromebook/Crostini compatibility handling (force X11 + scaling defaults when detected).

### Changed
- Background labels setting replaced free-text input with fixed special label selection.
- Batch size no longer exposed in settings; auto-batch selection drives inference.
- Language selection list stays stable and uses native labels (Nederlands, English, etc.).
- macOS bundling and release workflows updated for current runners and Feedie.app output.

### Fixed
- Context-menu export no longer triggers on hover; it requires a click (issue #2).
- Manifest fetch/update errors localized consistently.
- Thumbnail grid scroll behavior now enables immediate interaction on new galleries.

### Performance
- Faster preprocessing via fast_image_resize and zune-jpeg decode path.
- Pipeline overlap for preprocessing + inference to reduce idle time.
