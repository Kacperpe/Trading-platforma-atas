# Audyt bezpieczeństwa — Flowsurface (upstream `50c06ca`, 27.10.2026)

**Werdykt: kod jest bezpieczny do uruchamiania i dalszego rozwoju.** Nie znaleziono niczego, co wyglądałoby na złośliwe: brak telemetrii, brak ukrytych połączeń, brak wbudowanych kluczy, brak kodu wykonującego zlecenia. Są natomiast zależności z publicznymi podatnościami (część naprawiona w tym PR) i dwie rzeczy do poprawienia w przyszłości.

## Co sprawdziłem

| Obszar | Wynik |
|---|---|
| `build.rs` | Tylko `git rev-parse`/`git tag` (wersja) i zasoby ikony na Windows. Nic nie pobiera z sieci. ✅ |
| Skrypty i CI (`scripts/`, `.github/workflows`) | Standardowe budowanie, pakowanie i podpis. Bez pobierania zewnętrznych binarek w czasie budowania aplikacji. ✅ |
| Połączenia sieciowe (grep wszystkich URL w kodzie) | Wyłącznie publiczne API giełd: Binance, Bybit, OKX, Hyperliquid, MEXC oraz `data.binance.vision`. Jedyny inny host to `api.flowsurface.com` — katalog wskaźników, tylko gdy użytkownik go otworzy. Zgodne z `PRIVACY.md`. ✅ |
| Telemetria / analityka | Brak. ✅ |
| Sekrety w kodzie | Brak kluczy, tokenów i haseł. Token serwera danych i hasło proxy trafiają do systemowego keychaina. ✅ |
| Obsługa zleceń / pieniędzy | Brak — aplikacja tylko czyta dane rynkowe, nie ma kodu podpisującego zapytania (HMAC) ani składającego zleceń. ✅ |
| Wykonywanie procesów | Brak `Command::new` w aplikacji (poza `build.rs`). ✅ |
| Własne wskaźniki (`.ind`) | Interpretowany język z limitami zasobów, a nie natywne wtyczki (brak `dlopen`/WASM). Wgranie cudzego wskaźnika nie daje mu dostępu do systemu. ✅ |
| `unsafe` | 3 miejsca, wszystkie to `sonic_rs::to_object_iter_unchecked` na wiadomościach websocket z giełd (Bybit, Binance, MEXC). ⚠️ |
| TLS | Klient „serwera danych” akceptuje nieważne certyfikaty (`danger_accept_invalid_certs`). ⚠️ |
| Zależności (599 crate'ów vs baza RustSec) | 29 wpisów, w tym 17 podatności — szczegóły niżej. ⚠️ |
| Kompilacja i testy | `cargo check --workspace` ✅, testy `indicator-engine`: 69 ok, 0 błędów ✅ |

## Zależności z podatnościami

Naprawione w tym PR (`cargo update`, bez zmian w kodzie, build przechodzi):

- `rustls` 0.23.45 → 0.23.46 — 4 podatności (m.in. downgrade wymiany kluczy, wyciek sekretów w `Debug`)
- `quinn-proto` 0.11.15 → 0.11.19 — 5 podatności DoS w QUIC
- `rkyv` 0.8.16 → 0.8.18 — 3 podatności deserializacji
- `tokio`, `anyhow`, `memmap2`, `rustix` — poprawki „unsound”

Pozostałe, ocenione jako **niegroźne w tym projekcie**:

- `zip` 8.6.0 — wszystkie 4 podatności dotyczą `extract()`. Aplikacja tylko czyta CSV przez `by_index()` z archiwów Binance, więc problem jej nie dotyczy. Poprawka wymaga podbicia wersji głównej.
- `quick-xml` — używany tylko przez `wayland-scanner` podczas kompilacji, nie na danych z sieci.
- `arrow-data` 59.1 — problem z tablicami RunEndEncoded. Dotyczy tylko danych z twojego własnego serwera Arrow. Naprawić przy podbiciu Arrow do kolejnej wersji głównej.
- `lru`, `ttf-parser`, `bincode`, `paste`, `pin-utils` — pośrednie zależności biblioteki GUI `iced`, „unmaintained/unsound” bez praktycznego wektora ataku. Znikną z aktualizacją `iced`.

## Rekomendacje na później

1. **TLS serwera danych** — dodać opcję „wymagaj ważnego certyfikatu” (domyślnie włączona dla adresów spoza sieci lokalnej) albo przypinanie certyfikatu. Obecnie przy zdalnym serwerze możliwy jest atak MITM na dane i token.
2. **`unsafe` w parserach websocket** — zamienić na wersję sprawdzającą (`to_object_iter`) i zmierzyć wydajność. Zysk z wersji unchecked jest mały, a błędny JSON z giełdy nie powinien móc wywołać UB.
3. **CI**: dodać `cargo audit` (lub `cargo deny`) do workflow, żeby nowe podatności wychodziły automatycznie.
4. **Przed dodaniem handlu**: klucze API tylko w keychainie, osobne klucze bez uprawnień do wypłat, whitelista IP na giełdzie.
