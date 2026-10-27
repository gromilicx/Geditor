# Homebrew formula

Posle svih pet uspešnih buildova, release workflow generiše `geditor.rb` iz stvarnih arhiva i njihovih SHA-256 kontrolnih suma. Formula se nalazi u `release-bundle` artefaktu i u draft GitHub Release-u.

Objavi release, pa kopiraj taj fajl u `Formula/geditor.rb` zasebnog repozitorijuma `homebrew-tap`. Ne koristi izmišljene SHA vrednosti ili URL-ove koji još nisu objavljeni.

Kompletni koraci su u [RELEASING.md](../../RELEASING.md). Generator je u [scripts/release.py](../../scripts/release.py).
