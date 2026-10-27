# GEditor 0.3

[Releases](https://github.com/gromilicx/Geditor/releases) · [Homebrew tap](https://github.com/gromilicx/homebrew-tap)

```sh
brew install gromilicx/tap/geditor
ge .
```

Windows: preuzmi odgovarajući ZIP sa Releases, raspakuj ga i dodaj folder u svoj PATH; pokretanje je `ge.exe .`. Linux ima posebne x64 i ARM64 GNU pakete (glibc 2.35+). Mac paket je za Apple Silicon.

Besplatan terminalski editor napisan u Rustu. Autori prikazani u aplikaciji: **Miodrag Gromilic and OpenAI**. Ratatui, Crossterm, Syntect i pravi PTY terminal. MIT ili Apache-2.0 licenca.

## Pokretanje na ovom Macu

```sh
ge .                   # trenutni direktorijum
ge ~/Projects/projekat # drugi direktorijum
ge src/main.rs         # postojeći fajl, ili novi fajl u postojećem direktorijumu
```

Instalacija: `/Users/grom/.local/bin/geditor`; `ge` je simbolička veza ka istom izvršnom fajlu. Editor se pokreće bez Rust/Cargo instalacije. Izvori ostaju u `/Users/grom/Downloads/forgedit-rust`.

Ako nema otvorenog fajla, sredina editora prikazuje datum i digitalni sat sa sekundama. Zatvaranje poslednjeg sačuvanog fajla vraća taj ekran.

## Interfejs

Gornji meniji: **File, Edit, View, Settings, Help**. Klik ili tastatura: levo/desno bira meni, gore/dole stavku, Enter izvršava, Esc zatvara. Kartice otvorenih fajlova mogu se birati mišem.

`Ctrl+Tab` menja fokus redom **Explorer → Editor → AI → Terminal → Explorer**. Isključeni paneli se preskaču. `Ctrl+Shift+Tab` menja smer. Ako terminal presreće te kombinacije, koristi **Ctrl+K, pa Ctrl+F**. Nema obaveznih funkcijskih tastera.

View meni uključuje/isključuje svaki od četiri panela i minimapu. `[+]` u naslovu proširuje aktivni panel na ceo radni prostor; ponovni izbor vraća raspored. `[x]` sakriva panel. Podrazumevani raspored ostaje sačuvan i može se vratiti. Na uskom ekranu vidi se fokusirani panel; promenom fokusa prikazuju se ostali uključeni paneli.

Minimapa prikazuje gustinu koda kroz ceo fajl, označava stvarni vidljivi opseg i trenutni red. Klik vodi na odgovarajući red. Kratki fajlovi se ne ponavljaju da popune visinu. Pregled koristi ograničen uzorak znakova po prikazanom redu i automatski se skriva kada nema dovoljno širine.

## Prečice

Kombinacija „Ctrl+K, Ctrl+O“ znači: pritisni Ctrl+K, otpusti, pa pritisni Ctrl+O.

| Prečica | Radnja |
|---|---|
| Ctrl+Tab / Ctrl+Shift+Tab | Sledeći / prethodni uključeni panel |
| Ctrl+K, Ctrl+F | Sledeći panel; rezerva za starije terminale |
| Ctrl+E | Glavni meni, van ugrađenog terminala |
| Ctrl+K, Ctrl+V | Glavni meni iz svakog panela |
| Ctrl+N / Ctrl+O | Novi fajl / otvori fajl ili direktorijum |
| Ctrl+P | Brzo otvaranje projektnih fajlova |
| Ctrl+S / Ctrl+W | Sačuvaj / zatvori fajl |
| Ctrl+F / Ctrl+G | Pretraži otvoreni fajl / ceo projekat |
| Ctrl+U / Ctrl+R | Undo / redo |
| Ctrl+A / Ctrl+C / Ctrl+X / Ctrl+V | Označi sve / kopiraj / iseci / nalepi iz internog clipboarda |
| Ctrl+Q | Izlaz; ponovi za odbacivanje nesnimljenih izmena |
| Ctrl+K, Ctrl+B | Explorer uključi/isključi |
| Ctrl+K, Ctrl+E | Editor uključi/isključi |
| Ctrl+K, Ctrl+A | AI uključi/isključi |
| Ctrl+K, Ctrl+T | Terminal uključi/isključi |
| Ctrl+K, Ctrl+M | Minimap uključi/isključi |
| Ctrl+K, Ctrl+Z | Proširi/vrati aktivni panel |
| Ctrl+K, Ctrl+R | Vrati podrazumevani raspored |
| Ctrl+K, Ctrl+C | Sakrij aktivni panel |
| Ctrl+K, Ctrl+S | Save As; odbija postojeći fajl |
| Ctrl+K, Ctrl+D / Ctrl+K, Ctrl+U | Sledeća / prethodna kartica fajla |
| Ctrl+K, Ctrl+N / Ctrl+K, Ctrl+P | Sledeće / prethodno poklapanje poslednje pretrage |
| Ctrl+K, Ctrl+O | Settings |
| Ctrl+K, Ctrl+H / Ctrl+K, Ctrl+I | Help / About |
| Ctrl+K, Ctrl+K | Prosledi Ctrl+K ugrađenom shell-u |

U ugrađenom terminalu Ctrl+C, Ctrl+D, Ctrl+L, Ctrl+E i ostale standardne kontrole prosleđuju se shell-u. Za komande aplikacije koristi Ctrl+K kombinacije ili prvo promeni fokus. Tab/Enter/Backspace se unutar Ctrl+K kombinacija pravilno prepoznaju i kada terminal njima kodira Ctrl+I/M/H. U editoru unutrašnji clipboard radi između svih otvorenih fajlova; paste iz sistemskog clipboarda dolazi preko terminalskog paste-a (na Macu obično Cmd+V).

## Settings

Otvori **Settings → Preferences** ili `Ctrl+K, Ctrl+O`. Izmene izgleda vide se odmah. `Ctrl+S` čuva i primenjuje; `Esc` odbacuje. Ako je otvoreno polje ili dropdown, prvi Esc zatvara samo to polje. Odsek menjaš pomoću Ctrl+Tab ili prenosivih Ctrl+N / Ctrl+P; moguć je i klik na naslov odseka.

- **Appearance:** Midnight, Light, Dracula i Terminal teme; Auto/true color/256/16/monochrome; posebna boja teksta, pozadine i akcenta; ASCII ivice; veliki sat; tab širina; brojevi redova.
- **Panels:** početna vidljivost svih panela i minimape; širina sidebara, AI i minimape; procenat visine terminala. View promene važe za trenutnu sesiju, a Settings čuva početni raspored.
- **AI:** dropdown profila ključa, nazivi profila, maskirani unos ključa, dropdown modela, ručni ID modela, osvežavanje dostupnih modela, API base URL i uključivanje sadržaja aktivnog fajla u poruku.
- **General:** ime autora za About.

**Veličinu i porodicu fonta određuje terminalski emulator.** Standardni TUI protokol nema prenosivu kontrolu fontova. U Settings je to jasno prikazano; font i zoom menjaš u podešavanjima terminala (na Macu obično Cmd +/-). Širine i visine panela podešavaju se u aplikaciji.

Na macOS/Linux sistemima podešavanja se čuvaju u `~/.config/geditor/settings.json`, a na Windowsu u `%APPDATA%\GEditor\settings.json`. `XDG_CONFIG_HOME` može da promeni osnovni direktorijum. Za izolovan profil može se zadati `GEDITOR_CONFIG_DIR`. Na Unix sistemima direktorijum ima dozvole 0700, a fajl 0600. **Ključevi su lokalni tekst u tom privatnom fajlu, nisu šifrovani niti šalju se tokom samog podešavanja.** Mogu se koristiti i samo iz `OPENAI_API_KEY`, izborom Environment profila. Unos novog ključa je maskiran; prazan unos briše ključ iz izabranog profila.

## AI modeli i ključevi

1. U Settings/AI izaberi Environment ili jedan od tri profila (Personal/Work/Other; nazivi se mogu promeniti).
2. Za lokalni profil unesi ključ.
3. `Refresh models` šalje izričit GET zahtev na `/models`; sadržaj fajla se ne šalje. Zatim izaberi tekstualni model u dropdownu. Ručni ID je takođe moguć.
4. Ctrl+S čuva podešavanja. Enter u AI panelu šalje poruku izabranim ključem i modelom.

Modeli se učitavaju prema [zvaničnom Models API-ju](https://developers.openai.com/api/reference/resources/models/methods/list); katalog može sadržati i modele za druge namene, zato biraj model koji podržava tekstualni Responses API. Osnovni URL je `https://api.openai.com/v1`; prilagođeni endpoint mora podržavati Responses API. HTTPS je obavezan osim za localhost. HTTP preusmeravanja su isključena.

Sadržaj fajla (do 16 KB, uz ispravne UTF-8 granice) šalje se samo kada ti pošalješ chat poruku i uključen je Include file context. Slanje poruke čuva poslednjih do 12 poruka za kontekst i koristi `store: false`. Editor ne primenjuje AI predloge automatski. Aplikacija je besplatna; API usluga može imati sopstvenu cenu.

## Markdown uređivanje i live preview

```sh
ge README.md
```

`.md`, `.markdown`, `.mdown`, `.mkd` i `.mkdn` fajlovi (i velika slova u ekstenziji) imaju uređivanje izvornog teksta i formatirani pregled. Pregled se podrazumevano otvara uz kod; početno ponašanje menja se u Settings → Panels → Markdown on .md open.

| Prečica | Radnja |
|---|---|
| Ctrl+K, Ctrl+W | Uključi/isključi podeljeni Markdown pregled |
| Ctrl+K, Ctrl+L | Pregled preko celog radnog prostora; ponovo vraća podeljeni prikaz i raniji raspored |
| Ctrl+K, Ctrl+Y | Promeni fokus između izvornog teksta i pregleda |

Iste komande su u View meniju. Klik bira fokus. U pregledu rade gore/dole, PageUp/PageDown, Home/End i točkić miša. Esc vraća fokus na uređivanje. Sam pregled ne prihvata kucanje ili paste kao izmenu dokumenta; Ctrl+S i dalje snima izvornu Markdown datoteku. Pri uređivanju pregled prati aktivni deo dokumenta, a dok je fokus u pregledu može se nezavisno skrolovati.

Podržani su CommonMark/GFM naslovi, pasusi, bold/italic/precrtano, inline kod i ograđeni blokovi koda, liste i checkliste, citati, horizontalne linije, tabele, linkovi i reference fusnota. Unicode i uvlačenje koda se čuvaju. Pregled se osvežava iz buffera, uključujući nesnimljene izmene, undo i redo; rezultat se kešira dok su tekst, širina i tema isti.

Ovo je **terminalski pregled**. Slike imaju opis i putanju, linkovi prikazuju i odredište, a HTML ostaje tekst. Ne izvršavaju se HTML/JavaScript niti se automatski preuzimaju slike ili drugi resursi. CSS, vizuelni prikaz slika i matematičko slaganje nisu podržani. Kod veoma velikih dokumenata pregled prikazuje do 20.000 formatiranih redova uz vidljivu poruku o limitu; izvorni fajl ostaje dostupan za uređivanje u celosti.

## Precizna pretraga

`Ctrl+F` pretražuje aktivni buffer, uključujući nesnimljene izmene. `Ctrl+G` pretražuje direktorijum projekta u pozadini i koristi aktuelne verzije otvorenih imenovanih buffera. Quick Open (`Ctrl+P`) traži po relativnoj putanji fajlova.

U popupu za kod:

| Prečica | Opcija |
|---|---|
| Ctrl+C | Razlikovanje velikih i malih slova |
| Ctrl+W | Cela reč; Unicode granice reči |
| Ctrl+R | Regex, ili doslovni tekst |
| Ctrl+D | Uključi skrivene fajlove |
| Ctrl+A | Uključi ignorisane i build foldere |
| Ctrl+T | Prebaci unos između upita i filtera fajlova |
| Ctrl+S | Pokreni/ponovi pretragu |
| Ctrl+U | Očisti trenutno polje |

Levo/desno, Home/End, Delete i Backspace uređuju upit. Filteri prihvataju glob izraze odvojene zarezom, npr. `*.rs,*.toml` ili `src/**`. Pretraga otvorenog fajla reaguje odmah; pretraga projekta se pokreće Enterom ili Ctrl+S. Dok posao traje, prozor ostaje dostupan; izmena upita i Esc otkazuju zastareli posao.

Rezultat prikazuje **relativni fajl:red:kolona**, sa numeracijom od 1 i kolonom u Unicode znakovima, plus isečak koda usmeren na poklapanje. Gore/dole, PageUp/PageDown ili miš biraju rezultat. Enter ili klik odmah otvara fajl, pomera kursor i označava tačan opseg teksta. Iz otvorenog koda Ctrl+K Ctrl+N/P nastavlja poslednju listu poklapanja. Pogrešan regex daje poruku greške bez rušenja. Pretraga je po pojedinačnim redovima; višeredni regex nije podržan.

Podrazumevano se poštuje `.gitignore`, a build/skriveni folderi preskaču. Git interni direktorijum `.git` se uvek preskače, kao i symlink direktorijumi. Broj pregledanih i preskočenih fajlova i dostizanje limita prikazuju se u popupu. Limiti radi odziva: 5.000 poklapanja, 20.000 fajlova, 100.000 prolaza, dubina 40, 64 MiB pročitanog teksta i 15 sekundi po projektu. Binarni/ne-UTF-8 fajlovi i fajlovi preko 2 MiB se preskaču.

## Izgradnja i provera

Rust 1.88+ i sistemski razvojni alati; na ovom Macu korišćen je Rust 1.97.1 i Apple Command Line Tools.

```sh
cargo fmt --check
cargo test --release --locked
cargo clippy --release --all-targets --locked -- -D warnings
cargo build --release --locked
install -m 755 target/release/geditor "$HOME/.local/bin/geditor"
ln -sfn geditor "$HOME/.local/bin/ge"
```

`./scripts/build.sh` radi provere i izgradnju. Windows ima `scripts/build.ps1`; druge platforme nisu potvrđene na ovom Macu. `Cargo.lock` čuva proverene verzije biblioteka.

Testovi pokrivaju čuvanje i CRLF, zaštitu od prepisivanja, nevažeće fajlove, Unicode bojenje i pretragu, fokus/panele, ključne Ctrl kombinacije, clipboard između fajlova, podešavanja i privatne dozvole. AI test koristi samo lokalni HTTP server sa lažnim ključem. Poseban test za izvoz prikaza je isključen iz redovnog pokretanja i korišćen je za ručni pregled interfejsa.

## Granice podrške

Za pun interfejs preporučeno je bar 80×24; svi paneli istovremeno traže više širine. Manji prozori prilagođavaju raspored. Potreban je interaktivan VT-kompatibilan terminal; `TERM=dumb` nije podržan. Testiran je ovaj Apple Silicon Mac, xterm-256color i 80×24 VT prikaz, uključujući sekvence tastature i miša. Nije moguće garantovati iste fontove ili prečice u svakom terminalskom emulatoru; za to postoje ASCII/ANSI modovi i rezervne prečice.

Ovo je MVP editor: nema LSP/autocomplete, debugger, Git diff ni praćenje spoljnih izmena fajlova. Otvaranje fajla je ograničeno na 2 MiB i 65.535 redova, bojenje na prvih 3.000 redova. Spoljne izmene se ne osvežavaju automatski; zatvori sačuvanu karticu pa ponovo otvori fajl. Stvarni pozivi plaćenom AI servisu nisu izvršavani tokom razvoja.

## Windows, Linux, Mac ARM i Homebrew distribucija

Pripremljeni su native release buildovi za Windows/Linux x64 i ARM64 i macOS Apple Silicon. Uputstvo za GitHub Releases, generisanje Homebrew formule i sopstveni tap je u [RELEASING.md](RELEASING.md). Objavljena verzija koristi pakete iz GitHub Actions native buildova za odgovarajući procesor. Detalji statusa provera dostupni su u Actions kartici repozitorijuma.
