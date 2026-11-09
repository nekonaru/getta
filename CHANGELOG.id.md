# Changelog

Semua perubahan penting Getta didokumentasikan di sini dan di `CHANGELOG.md`.
Versi terbaru di atas.

## v0.5.0

Unduhan yang terputus kembali, dan antrean bisa diatur ulang.

## Fitur baru

- **Pemulihan terputus.** Jika Getta ditutup atau aplikasi mati di tengah unduhan, saat dibuka tampil dialog untuk melanjutkan unduhan yang belum selesai dari file `.part`.
- **Urutan antrean manual.** Item menunggu punya tombol naik dan turun di manifest; scheduler menjalankannya sesuai urutanmu.
- **Pisahkan per bab.** Opsional: unduh bab-bab sebagai file terpisah (yt-dlp).
- **Live dari awal.** Opsional: putar live streaming dari awal (yt-dlp).

## v0.4.1

Polish native dan pengalaman yang lebih tenang dan cepat.

## Fitur baru

- **Tutup ke tray.** Opsi: tombol tutup menyembunyikan Getta ke tray alih-alih keluar; kembalikan kapan saja lewat ikon tray.

## Peningkatan

- **Chip status mesin.** Chip sidebar kini menampilkan status "memeriksa" sampai pengecekan mesin selesai, bukan "engine ready" yang menyesatkan.
- **Latar jendela.** Latar native jendela sekarang warna paper hangat, bukan hijau gelap yang tersisa.
- **String lebih rapi.** Placeholder tak terpakai dihapus dari berkas locale.

## v0.4.0

Halaman dukung tersedia dengan semua cara mendukung Getta, dan hyperlink eksternal kembali berfungsi.

## Fitur baru

- **Dukung Getta.** Halaman dukung baru dengan GitHub Sponsors, Saweria, SociaBuzz, dan BagiBagi. Buka dari sidebar, halaman Dukung, atau Tentang.

## Perbaikan bug

- **Hyperlink eksternal mati.** Tautan yang membuka browser sistem sekarang berfungsi di mana pun (Tentang, sidebar): GitHub, rilis, issues, dan tautan sponsor.

## Peningkatan

- **Thumbnail pustaka.** Kartu sekarang memudar dengan lembut di atas placeholder shimmer alih-alih muncul tiba-tiba.

## v0.3.3

Login situs kembali berfungsi, progres sekarang terlihat hidup, dan kasus-kasus edge dari lapangan diperbaiki.

## Fitur baru

- **Dukungan file cookie.** Pilih ekspor `cookies.txt` di Pengaturan, Mesin; file ini diutamakan daripada membaca cookie dari browser.
- **Progress bar agregat.** Layar antrean menampilkan satu progress bar keseluruhan di atas manifest, dihitung berdasarkan byte bila diketahui.

## Peningkatan

- **Error yang bisa dimengerti.** Kegagalan mesin dipetakan menjadi panduan yang bisa ditindaklanjuti, bukan dump stack yt-dlp mentah.
- **Tautan instance kedua.** Tautan getta:// yang dikirim ke launch kedua diteruskan ke aplikasi yang sedang berjalan, bukan diabaikan.

## Perbaikan bug

- **Kegagalan DPAPI Chrome 127+.** Edge dan Chrome tidak lagi bisa dibaca pihak ketiga; Getta kini membimbing pengguna dan menerima cookies.txt sebagai jalur resmi.
- **Resume menghormati concurrency.** Melanjutkan unduhan yang dijeda akan mengantrekannya sesuai batas, bukan langsung melompat.
- **Enqueue deep-link ganda.** Menjalankan aplikasi lewat getta:// saat aplikasi sedang berjalan bisa mengantrekankan URL yang sama dua kali.

## v0.3.2

Tidak ada lagi jendela console liar saat Getta berjalan.

## Perbaikan bug

- **Console flash.** Setiap pengecekan mesin memanggil proses console di Windows, sehingga terminal jendela muncul sekejap saat aplikasi dibuka, setiap buka Settings atau cek mesin, dan saat pindah halaman. Semua proses anak sekarang dibuat tanpa jendela console.

## v0.3.1

Tidak ada lagi terminal di belakang aplikasi.

## Perbaikan bug

- **Terminal liar di Windows.** Build rilis kini mendeklarasikan subsistem GUI Windows, jadi tidak ada console window terbuka di belakang Getta dan menutupnya tidak lagi mematikan aplikasi.

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
