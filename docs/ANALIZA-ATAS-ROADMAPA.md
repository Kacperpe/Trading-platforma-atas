# Trading-platforma-atas — analiza luk względem ATAS i plan projektu

Baza: fork Flowsurface (upstream `50c06ca`, wersja 0.9.0, ~122 tys. linii Rusta, GPLv3).
Cel projektu: platforma order flow klasy ATAS, najpierw na krypto, potem na futures CME.
Zasada: funkcje odtwarzamy na podstawie publicznie znanych koncepcji order flow i własnej specyfikacji. Nie używamy kodu, plików ani zasobów ATAS (clean room).

---

## 1. Decyzja: fork czy od zera?

**Rekomendacja: fork Flowsurface.**

| | Fork Flowsurface | Od zera |
|---|---|---|
| Czas do poziomu „to, co jest dziś” | 0 | ok. 6–9 miesięcy po godzinach |
| Render GPU (wgpu), heatmapa, footprint | gotowe i szybkie | najtrudniejsza część projektu |
| Adaptery 5 giełd, reconnect, backfill | gotowe | 1–2 mies. |
| Język wskaźników z limitami zasobów | gotowy | opcjonalny |
| Licencja | **GPLv3** — kod forka musi być otwarty, jeśli go dystrybuujesz | dowolna |
| Język | Rust (trzeba się go nauczyć) | dowolny |

Jedyny powód, żeby pisać od zera: zamknięty, komercyjny produkt na sprzedaż. Wtedy GPL blokuje fork. Dla własnego użytku, nauki albo projektu open source fork wygrywa zdecydowanie. **Punkt decyzyjny:** jeśli kiedyś pojawi się plan sprzedaży, przepisać tylko warstwę UI/rdzeń — na to będzie czas po zweryfikowaniu pomysłu na forku.

---

## 2. Co już mamy vs ATAS

Legenda: ✅ jest · 🟡 częściowo · ❌ brak

### Wykresy i order flow

| Funkcja | Status | Uwagi |
|---|---|---|
| Footprint (bid×ask na poziomie ceny) | ✅ | różne metody klastrowania |
| Imbalance na footprincie | ✅ | konfigurowalny próg |
| Stacked imbalance (kilka z rzędu) jako strefy | ❌ | proste rozszerzenie istniejącego imbalance |
| Naked POC | ✅ | |
| Volume Profile (widoczny zakres i cykliczny) | ✅ | |
| TPO / Market Profile | ✅ | |
| Heatmapa księgi (historyczny DOM) | ✅ | |
| DOM / ladder | 🟡 | tylko podgląd, bez handlu |
| Time & Sales | ✅ | |
| Smart Tape (sklejanie transakcji w zlecenia) | ❌ | agregacja po timestampie/stronie/cenie |
| Big Trades | ✅ | |
| Cumulative Volume Delta | ✅ | wskaźnik `.ind` |
| VWAP, anchored VWAP, Open Interest, RSI, BB, MA | ✅ | wskaźniki `.ind` |
| Panel statystyk pod świecami (delta, wolumen, min/max delta, czas) | 🟡 | dane są w footprincie, brakuje tabeli pod wykresem |
| Wykrywanie icebergów / „trzymanych” poziomów | ❌ | heurystyka: wolumen > widoczna płynność |
| Śledzenie dużych zleceń w księdze (pojawienie/zdjęcie) | ❌ | na danych L2 z heatmapy |
| Wykres świecowy czasowy i tickowy | ✅ | |
| Wykresy range / renko / wolumenowe / deltowe | ❌ | nowy wariant agregacji obok `Basis::Tick` |
| Narzędzia rysowania: linie, prostokąt, pomiar | ✅ | |
| Fibonacci, kanały, tekst, profil na zaznaczeniu | ❌ | |

### Platforma

| Funkcja | Status | Uwagi |
|---|---|---|
| Wiele okien/monitorów, layouty, motywy | ✅ | |
| Dźwięki z taśmy | ✅ | |
| Alerty (cena, big trade, delta, poziom) | ❌ | dźwięk + powiadomienie systemowe |
| Market Replay (odtwarzanie sesji z kontrolą prędkości) | ❌ | jest tylko odbudowa historycznej heatmapy |
| Nagrywanie własnych danych tick + L2 | 🟡 | flowsurface-server zapisuje trade'y do DuckDB, brak L2 |
| Własne dane historyczne z serwera (Arrow) | ✅ | dla footprintu |
| Handel z wykresu i DOM, SL/TP, OCO | ❌ | |
| Paper trading / symulator | ❌ | |
| Dziennik i statystyki transakcji | ❌ | |
| API strategii / backtest | 🟡 | DSL tylko do wskaźników |
| Giełdy krypto (Binance, Bybit, OKX, Hyperliquid, MEXC) | ✅ | |
| Futures CME / Eurex | ❌ | Databento (historia), Rithmic/CQG (live) |

**Podsumowanie:** około 60–65% zakresu „analitycznego” ATAS już działa. Brakuje głównie: Smart Tape, replay, alertów, analizy księgi (icebergi), dodatkowych typów świec oraz całego obszaru handlu i danych CME.

---

## 3. Plan projektu (jak prowadziłbym to jako dyrektor projektu)

Założenie: 1 osoba, 10–15 h tygodniowo (tryb studencki). Wszystkie terminy w tygodniach od startu.

### Faza 0 — Fundamenty (tydz. 1–2)
- Budowanie lokalne na Windows, przejście przez architekturę: `exchange/` (adaptery), `data/` (modele, agregacje), `src/screen` (UI).
- CI w forku + `cargo audit`.
- Workflow synchronizacji z upstreamem (merge co 2–4 tyg.), żeby nie tracić ich poprawek.
- **Kamień milowy:** własny build działa, pierwszy drobny commit (np. nazwa/ikona).

### Faza 1 — Szybkie wygrane order flow (tydz. 3–8)
1. Smart Tape — agregacja w Time & Sales (1 tydz.)
2. Panel statystyk pod świecami (1–2 tyg.)
3. Stacked imbalance jako strefy (1 tydz.)
4. Alerty: cena, big trade, próg delty (1–2 tyg.)
5. Rysowanie: Fibonacci, tekst, kanał (1 tydz.)
- **Kamień milowy:** codzienna analiza BTC/ETH możliwa bez ATAS.

### Faza 2 — Nagrywanie i Market Replay (tydz. 9–14)
- Rejestrator trade'ów i snapshotów/delt L2 do plików (Arrow/Parquet), rozszerzenie flowsurface-server.
- Odtwarzacz udający adapter giełdy: start/pauza/prędkość ×1–×50, skok do czasu.
- **Kamień milowy:** odtworzenie wczorajszej sesji z footprintem i heatmapą.

### Faza 3 — Dane CME (tydz. 12–18, częściowo równolegle)
- Konwerter Databento (trades) → schemat Arrow `ts, price, qty, is_sell` → serwer. Footprint na ES/NQ z historii.
- Nowy `Venue::Cme` + adapter MBP-10 dla heatmapy i DOM.
- Budżet: dane historyczne pay-as-you-go (kilkadziesiąt USD na start).
- **Kamień milowy:** analiza ES/NQ z poprzednich sesji na footprincie i heatmapie.

### Faza 4 — Handel na krypto (tydz. 19–28)
1. Paper trading: symulator fill na podstawie taśmy, pozycja/PnL na wykresie.
2. Bybit API: zlecenia limit/market, SL/TP, OCO, przesuwanie zleceń z DOM.
3. Bezpieczeństwo: klucze w keychainie, limity pozycji, potwierdzenia, kill switch.
4. Dziennik transakcji i statystyki.
- **Kamień milowy:** pierwszy tydzień handlu na małym koncie wyłącznie z własnej platformy.

### Faza 5 — Zaawansowane (od tyg. 29)
- Wykrywanie icebergów i śledzenie dużych zleceń w księdze.
- Wykresy range/renko/wolumenowe.
- CME live przez Rithmic lub CQG (licencje, certyfikacja, opłaty giełdowe).
- Backtest prostych strategii na nagranych danych.

### Harmonogram w skrócie

```
Tydz.   1   4   8   12  16  20  24  28  32+
F0     ██
F1        ██████
F2                ██████
F3                  ███████
F4                         ██████████
F5                                    ████→
```

### Ryzyka

| Ryzyko | Wpływ | Co z tym robimy |
|---|---|---|
| Krzywa nauki Rusta | wolniejszy start | Faza 1 to małe, izolowane zadania — dobre do nauki |
| Rozjazd z upstreamem | trudne merge | zmiany w osobnych modułach, regularny merge |
| Licencja GPLv3 | brak zamkniętej sprzedaży | świadoma decyzja w pkt. 1 |
| Koszty danych CME | budżet | najpierw historia pay-as-you-go, live dopiero gdy projekt się sprawdzi |
| Błąd w module zleceń | realne straty | paper trading najpierw, limity, testy, małe konto |
| Licencje Rithmic/CQG | blokada CME live | odłożone do Fazy 5, nie blokuje reszty |

---

## 4. Pierwsze kroki (do zrobienia teraz)

1. Zmergować ten PR (aktualizacje bezpieczeństwa + dokumentacja).
2. Zbudować lokalnie: `cargo run --release`.
3. Zacząć Fazę 1 od Smart Tape — najmniejszy, samodzielny moduł.
