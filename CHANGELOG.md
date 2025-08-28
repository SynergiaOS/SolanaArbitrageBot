# Changelog
# ... existing code ...
## [0.1.1] - 2025-08-26
- Stabilny wariant monitor-only: build i testy bez domyślnych cech z explicytną cechą monitor.
- CI: wymuszone komendy monitor-only (clippy/test/build) oraz walidacja bramek zależności cargo tree.
- Release: uruchamiany na tagach v*, artefakty monitor-only w GitHub Release.
- Docker: dodany ARG CARGO_FEATURES=monitor; budowanie z --no-default-features --features monitor; publikacja obrazów GHCR z tagami latest oraz :monitor i :SHA-monitor.
- Wersja pakietu podbita do 0.1.1.

## [0.1.0]
- Wersja początkowa.