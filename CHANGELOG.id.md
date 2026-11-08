# Changelog

Semua perubahan penting Getta didokumentasikan di sini dan di `CHANGELOG.md`.
Versi terbaru di atas.

## v0.2.0

Desktop terasa seperti aplikasi native. Mobile tidak lagi direncanakan.

## Fitur baru

- **System tray.** Getta menambahkan ikon di system tray: klik kiri membuka/menyembunyikan window, klik kanan membuka menu dengan Tampilkan dan Keluar. Label mengikuti bahasa yang diatur.

## Peningkatan

- **Hanya desktop.** Rencana mobile dibatalkan; dokumentasi kini menyatakan Windows, macOS, dan Linux saja.
- **Versi di Tentang.** Layar Tentang membaca versi dari manifest paket, bukan string tetap.
- **Repository lebih ramping.** Set ikon Android dan iOS yang tidak terpakai dari generasi awal dihapus.

## v0.1.1

Perbaikan untuk layar kosong saat pertama dibuka.

## Perbaikan bug

- **Window kosong.** Aplikasi crash saat startup karena modul bahasa memakai rune Svelte 5 di file TypeScript biasa yang tidak pernah dikompilasi. Modulnya sekarang dikompilasi sebagai modul Svelte dan aplikasi tampil normal.

## v0.1.0

Rilis publik pertama. Tempel tautan, ambil filenya.

## Fitur baru

- **Omnibox di Beranda.** Satu kolom input menangani tautan tunggal, banyak tautan sekaligus, dan URL file langsung, dengan deteksi platform, pratinjau media, pemilih kualitas, serta mode audio/video saja.
- **Mesin unduhan.** Unduhan lokal berjalan lewat yt-dlp dan FFmpeg. Getta bisa memasang dan memverifikasi keduanya sendiri di Windows dan macOS; di Linux memakai paket dari distro.
- **Antrean dengan resume.** Antrean persisten dengan jeda, lanjutkan, batal, coba ulang dengan backoff, kecepatan live, ETA, dan label platform per situs. Jumlah unduhan bersamaan, jumlah koneksi, dan coba ulang bisa diatur.
- **Riwayat dan tampilkan.** Unduhan selesai terdaftar, bisa diunduh ulang, ditampilkan di file manager, atau dihapus.
- **UI dwibahasa.** Bahasa Inggris dan Bahasa Indonesia di semua layar, plus tema terang, gelap, dan sistem.
- **Tanpa akun, tanpa telemetri.** Getta tidak menghubungi server mana pun. Unduhan langsung dari situs ke diskmumu lewat mesin unduhan.
