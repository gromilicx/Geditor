# GEditor: Windows, Linux, Mac ARM i Homebrew

Održavanje izdanja za https://github.com/gromilicx/Geditor i Homebrew tap https://github.com/gromilicx/homebrew-tap.

## Platforme

| Sistem | Procesor | Rust target | Runner | Arhiva |
|---|---|---|---|---|
| macOS | Apple Silicon / ARM64 | aarch64-apple-darwin | macos-14 | .tar.gz |
| Linux | Intel/AMD 64-bit | x86_64-unknown-linux-gnu | ubuntu-22.04 | .tar.gz |
| Linux | ARM64 | aarch64-unknown-linux-gnu | ubuntu-22.04-arm | .tar.gz |
| Windows | Intel/AMD 64-bit | x86_64-pc-windows-msvc | windows-2022 | .zip |
| Windows | ARM64 | aarch64-pc-windows-msvc | windows-11-arm | .zip |

Izvršni fajl je `geditor`/`ge` na Macu i Linuxu, odnosno `geditor.exe`/`ge.exe` na Windowsu. Korisnicima ne treba Rust. Jedan Mac izvršni fajl ne radi na drugim operativnim sistemima, čak ni kada svi imaju ARM64 procesor.

Mac paket cilja macOS 11+, dok sam Homebrew ima svoja ograničenja podržanih macOS verzija. Linux GNU paketi se grade na Ubuntu 22.04, pa zahtevaju glibc 2.35+; nisu namenjeni Alpine/musl sistemima ni 32-bitnim ARM uređajima. Za takve platforme je potreban poseban build. Windows release gradi statički C runtime radi jednostavnije distribucije; izvršavanje i testovi moraju proći na njegovom Windows runneru.

## Šta je pripremljeno

- `.github/workflows/release.yml`: pet zasebnih buildova, Rust testovi, Clippy, test izvršnog fajla i pakovanje.
- `scripts/release.py`: provera oznake verzije, binarnog formata i procesora; pakovanje; SHA-256; generisanje Homebrew formule iz završenih paketa.
- `scripts/test_release.py`: testovi alata za pakovanje. Sintetička zaglavlja koriste se samo za unit testove i nisu stvarni izvršni fajlovi.
- Redovna CI provera u `.github/workflows/rust.yml`.

`workflow_dispatch` pravi samo Actions artefakte. Push taga `v0.3.0` pokreće iste provere i tek posle uspeha svih pet platformi kreira **draft GitHub Release**. Draft se objavljuje odvojeno. Ne stvara niti menja Homebrew tap automatski.

## Prvi put: dva javna GitHub repozitorijuma

Predlog za postojeći GitHub nalog `gromilicx`:

1. `gromilicx/Geditor` — izvori i gotovi izvršni fajlovi.
2. `gromilicx/homebrew-tap` — Homebrew formula.

Imena su predlog, a sledeće komande su koraci za buduće objavljivanje, nisu već izvršene. Javni izvori i release fajlovi postaće vidljivi svima. Sopstveni tap može se koristiti bez prihvatanja u homebrew/core.

Iz direktorijuma projekta:

```sh
cd /Users/grom/Downloads/forgedit-rust
git init -b main
git add Cargo.toml Cargo.lock LICENSE README.md RELEASING.md .gitignore src scripts examples packaging .github
git commit -m "Prepare GEditor release builds"
gh repo create gromilicx/Geditor --public --source . --remote origin --push
```

Ako odabereš druga imena, zameni ih u komandama. Release workflow sam uzima stvarni `github.repository`, pa generisana formula koristi odgovarajući URL.

## Provera i prva verzija

Najpre u GitHub Actions pokreni **Release binaries → Run workflow**, sa grane main. To proverava sve sisteme bez pravljenja release-a.

Kada svih pet platformi prođe, označi verziju. Tag mora tačno odgovarati `[package].version` u Cargo.toml:

```sh
git tag v0.3.0
git push origin v0.3.0
```

Posle završetka preuzmi artefakt `release-bundle`, ili pregledaj draft na GitHub Releases. Sadrži:

```text
geditor-0.3.0-macos-arm64.tar.gz
geditor-0.3.0-linux-x64.tar.gz
geditor-0.3.0-linux-arm64.tar.gz
geditor-0.3.0-windows-x64.zip
geditor-0.3.0-windows-arm64.zip
SHA256SUMS
geditor.rb
*.manifest.json
```

Neće se napraviti kompletan bundle ako nedostaje neka platforma ili checksum nije ispravan. Draft se može preuzeti autentifikovanim GitHub CLI-jem:

```sh
gh release download v0.3.0 --repo gromilicx/Geditor --dir dist/published
```

Pregledaj i praktično isprobaj pakete na tim sistemima. CI pokriva kompilaciju, automatizovane testove i CLI `--version`/`--help`; nije zamena za interaktivnu proveru editora, ugrađenog terminala i AI mreže na svakom sistemu.

Objavljivanje završenog draft-a:

```sh
gh release edit v0.3.0 --repo gromilicx/Geditor --draft=false
```

Ovo je javno objavljivanje. Homebrew URL-ovi neće raditi dok release nije objavljen. Kod kasnijih verzija ažuriraj Cargo.toml, osveži Cargo.lock kroz Cargo, sačuvaj izmene u Gitu i napravi novi tag. CLI i About uzimaju verziju iz Cargo paketa.

## Homebrew tap

Napravi zaseban lokalni direktorijum za tap i kopiraj **generisanu** formulu iz završenog release-a:

```sh
mkdir -p /Users/grom/Downloads/homebrew-tap/Formula
cp dist/published/geditor.rb /Users/grom/Downloads/homebrew-tap/Formula/geditor.rb
cd /Users/grom/Downloads/homebrew-tap
git init -b main
git add Formula/geditor.rb
git commit -m "Add GEditor 0.3.0"
gh repo create gromilicx/homebrew-tap --public --source . --remote origin --push
```

Formula bira Mac ARM64 ili odgovarajući Linux paket i proverava stvarni SHA-256. Instalira `geditor` i prečicu `ge`; ne preuzima Rust, jer koristi gotove izvršne fajlove. Windows korisnici koriste ZIP, jer Homebrew nije njihov instalacioni put.

Korisnik zatim instalira jednom komandom:

```sh
brew install gromilicx/tap/geditor
ge .
```

Ili odvojeno:

```sh
brew tap gromilicx/tap
brew install geditor
```

Provera objavljene formule i kasnije nadogradnje:

```sh
brew test gromilicx/tap/geditor
brew update
brew upgrade geditor
```

Za novu verziju prekopiraj njen novi `geditor.rb` u postojeći tap, commit i push. Ne zadržavaj stare kontrolne sume uz novi URL.

## Tvoja postojeća ručna instalacija

Trenutni `ge` je u `~/.local/bin`, koji je ispred Homebrew putanje. Posle buduće brew instalacije proveri:

```sh
type -a ge
type -a geditor
```

Dok je prvi rezultat `~/.local/bin/ge`, pokreće se ručna verzija, a ne Homebrew verzija. Za migraciju treba ukloniti ili preimenovati stare ručne alias/symlink fajlove i sačuvati postojeći executable kao rezervu. Podešavanja u `~/.config/geditor` ostaju odvojena od instalacije. U ovoj pripremi ručna instalacija i PATH nisu prebacivani na Homebrew.

## Ručno pakovanje Mac ARM verzije

```sh
cargo build --release --locked
python3 scripts/release.py package --target aarch64-apple-darwin --binary target/release/geditor --out dist
```

`finalize` se pokreće tek kada su u istom direktorijumu svih pet stvarnih paketa i njihovih manifest fajlova:

```sh
python3 scripts/release.py finalize --directory dist --repository gromilicx/Geditor
```

Generator ne izmišlja nedostajuće platforme ili SHA vrednosti.

Za direktnu distribuciju većem broju korisnika može se kasnije dodati Apple Developer ID potpisivanje/notarizacija i Windows potpisivanje. Sertifikati nisu deo ove pripreme; sadašnji workflow ne tvrdi da pravi takve potpisane pakete.

## Zvanična dokumentacija

- [Rust platforme](https://doc.rust-lang.org/rustc/platform-support.html)
- [GitHub runneri i procesori](https://docs.github.com/en/actions/reference/runners/github-hosted-runners)
- [GitHub Releases](https://docs.github.com/en/repositories/releasing-projects-on-github/managing-releases-in-a-repository)
- [Homebrew tap](https://docs.brew.sh/How-to-Create-and-Maintain-a-Tap)
- [Homebrew Formula Cookbook](https://docs.brew.sh/Formula-Cookbook)
