# GEditor — Markdown demo

Uređuj ovaj fajl i prati **live preview**, čak i pre čuvanja.
Podržani su *italic*, **bold**, ~~precrtano~~ i `inline code`, uključujući čćšđž i 世界.

## Prečice

- Ctrl+K, pa Ctrl+W: uključi/isključi pregled uz izvor.
- Ctrl+K, pa Ctrl+L: pregled preko celog prostora / povratak na podeljeni prikaz.
- Ctrl+K, pa Ctrl+Y: promeni fokus između izvora i pregleda.
- U pregledu: strelice, PageUp/PageDown, Home/End i točkić miša.
- Ctrl+S: sačuvaj Markdown izvor.

## Checklist

- [x] Uređivanje Markdown izvora
- [x] Pregled nesnimljenih izmena
- [x] Tabele, citati i blokovi koda
- [ ] Napiši svoj sledeći dokument

## Tabela

| Funkcija | Status |
|---|---|
| Unicode | čćšđž — radi |
| Live preview | Uključen |
| Čuvanje | Ctrl+S |

## Kod

```rust
fn main() {
    let poruka = "Zdravo iz GEditora!";
    println!("{poruka}");
}
```

> Pregled radi lokalno iz otvorenog buffera. Izvorna Markdown datoteka se ne pretvara u drugi format.

1. Otvori fajl.
2. Promeni tekst.
3. Uporedi izvor i formatirani pregled.

---

[Uputstvo](../README.md)

![Opis ilustracije](illustration.png)

Slike se u terminalu prikazuju opisom i putanjom; ne preuzimaju se automatski.
HTML ostaje tekst, a skripte se ne izvršavaju.
