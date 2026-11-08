# Changelog

Semua perubahan penting Getta didokumentasikan di sini dan di `CHANGELOG.md`.
Versi terbaru di atas.

## v0.3.0

Redesain editorial dan kumpulan fitur besar.

## Fitur baru

- **Redesain editorial.** Sidebar baru bernomor, palet bone hangat dengan aksen cobalt, tipografi Fraunces, daftar unduhan bergaya manifest.
- **Pustaka.** Unduhan selesai tampil sebagai grid; putar video, audio, dan gambar langsung di aplikasi, atau tampilkan di file manager.
- **Command palette.** Ctrl/Cmd+K membuka launcher fuzzy: lompat ke halaman mana pun, atau jalankan aksi seperti jeda semua, lanjutkan semua, bersihkan selesai, dan unduh dari clipboard.
- **Alat.** Meja kerja lokal: Video ke MP3, Video ke GIF, Kompres video dengan H.264, ditenagai FFmpeg yang sudah kamu punya.
- **Pemilih daftar putar.** Playlist dianalisis dengan daftar ceklis; unduh semuanya atau hanya entri yang kamu mau, dengan format terpilih.
- **Onboarding.** Wizard peluncuran pertama mengatur bahasa dan tema, memasang mesin unduhan, dan memilih folder tujuan.
- **Progres real-time.** Sparkline kecepatan, ETA live, fase transfer, dan strip agregat di atas antrean.
- **Global hotkey.** Ctrl/Cmd+Shift+D mengambil dari clipboard dari mana pun.
- **Deep link.** Tautan getta:// membuka Getta dan mengantrekan URL-nya.
- **Autostart.** Opsi menjalankan Getta saat sistem menyala di Pengaturan.

## Peningkatan

- Tampilan antrean kini terbagi (Dalam perjalanan, Menunggu, Terkirim, Gagal) dengan jumlah per bagian.
- Baris statistik di Beranda: file tersimpan dan total byte yang diambil.
- Notifikasi toast lebih sederhana di tengah bawah.

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
