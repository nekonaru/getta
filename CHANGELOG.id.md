# Changelog

Semua perubahan penting Getta didokumentasikan di sini dan di `CHANGELOG.md`.
Versi terbaru di atas.

## v1.0.0

Titik stabil: API lokal, perintah CLI, dan integrasi akhir.

## Fitur baru

- **API lokal.** Server opsional hanya loopback pada port 17842 dengan token yang dihasilkan otomatis. Menerima `POST /add` dengan `{"url":"..."}` dan `GET /status`, serta tidak pernah membuka antarmuka publik.
- **Perintah CLI.** `getta <url>` atau `getta add <url>` mengantrekan link ke aplikasi yang berjalan. `getta pause` dan `getta resume` mengendalikan semua unduhan.
- **Jembatan pintasan keyboard.** Perintah CLI pause dan resume dikirim ke antarmuka lewat event, sehingga jendela yang berjalan tetap menjadi sumber kebenaran.

## v0.20.0

Otomasi lanjutan: feed RSS, interval follow per kanal, monitor clipboard, profil pengaturan, dan filter antrean yang lebih pintar.

## Fitur baru

- **Feed RSS.** Tambahkan URL feed di Pengaturan. Entri baru diurai dan diantrekan otomatis, dengan daftar sudah dilihat dan catatan audit lokal.
- **Interval follow per kanal.** Pilih 15 menit, 1 jam, 6 jam, atau 1 hari untuk tiap kanal yang diikuti, bukan satu interval enam jam global.
- **Monitor clipboard.** Saat aktif, Getta memantau clipboard dan mengantrekan link begitu link baru muncul.
- **Profil pengaturan.** Simpan pengaturan saat ini sebagai profil bernama dan kembalikan ke profil itu kapan saja.
- **Filter grup dan statistik antrean.** Filter antrean berdasarkan grup dan lihat jumlah aktif, menunggu, selesai, gagal, serta total byte di atas daftar.

## v0.19.0

Update besar: unduhan langsung native, otomasi antrean, metadata pustaka yang kaya, dan alat data lokal.

## Fitur baru

- **Unduhan langsung native.** Link file http(s) kini mengalir lewat Getta sendiri, dengan resume `.part`, batas kecepatan per item, dan progres nyata. yt-dlp tidak diperlukan untuk file biasa.
- **Prioritas dan jam hening.** Beri item prioritas Rendah sampai Segera. Penjadwal memulai prioritas lebih tinggi dulu dan bisa menahan unduhan baru pada jam hening yang diatur.
- **Preset unduhan.** Simpan kualitas, mode, folder, template, prioritas, dan kecepatan sebagai preset, lalu pakai dari Beranda dalam satu klik.
- **Kebijakan retry yang bisa diatur.** Atur jeda awal dan pengali, bukan backoff eksponensial tetap.
- **Verifikasi checksum.** Masukkan SHA-256 yang diharapkan dan Getta memeriksa file selesai sebelum menandainya berhasil.
- **Notifikasi webhook.** Kirim peristiwa JSON ke endpoint lokal atau self-hosted saat unduhan selesai atau gagal.
- **Aksi setelah unduhan.** Otomatis memindahkan atau menyalin file selesai ke folder kedua.
- **Metadata pustaka.** Tambahkan tag, peringkat 1 sampai 5, catatan, favorit, dan penanda tonton nanti pada item selesai.
- **Ganti nama massal.** Ubah nama file terpilih dengan template yang memakai judul, situs, id, dan nama lama, aman dari benturan.
- **Analitik penyimpanan.** Lihat total byte terunduh dan rincian per situs di Pustaka.
- **Folder pintar.** Simpan pencarian pustaka sebagai definisi folder bernama di Pengaturan.
- **Cadangkan dan pulihkan.** Buat satu ZIP berisi pengaturan, riwayat, follow, dan pemulihan, lalu pulihkan kemudian.
- **Log audit.** Catatan lokal appended untuk enqueue, ganti nama, cadangan, dan aktivitas watch folder.
- **Pemeriksaan kesehatan.** Cek ruang disk dan apakah folder keluaran bisa ditulis dari Tentang.
- **Ekspor pustaka offline.** Ekspor entri selesai ke galeri HTML mandiri.
- **Warna aksen dan mode ringkas.** Pilih aksen kustom dan aktifkan tata letak lebih padat.
- **Watch folder.** Letakkan file `.txt` berisi link ke sebuah folder dan Getta mengantrekannya otomatis.
- **Pemendek tautan.** Shortener yang dikenal dibuka dulu sebelum analisis dan enqueue.
- **Aturan bersyarat.** Aturan unduhan kini juga bisa menetapkan prioritas atau memakai preset.

## v0.18.0

Unduhan terjadwal, impor link dari file, dan audit native putaran ketiga.

## Fitur baru

- **Unduhan terjadwal.** Atur waktu mulai di Beranda. Item menunggu di antrean dengan cap waktu yang terlihat dan bisa dimulai lebih awal dari barisnya. Jadwal tetap tersimpan saat aplikasi dibuka ulang lewat pemulihan.
- **Impor link dari file.** Beranda kini membaca file teks UTF-8 berisi link, menghapus duplikat, dan mengantrekan hingga 500 link sekaligus.

## Perbaikan bug

- **Link yang diteruskan tidak hilang lagi.** Link yang tiba sebelum antarmuka siap disimpan sementara lalu diputar ulang.
- **Semua link dari instance kedua diteruskan.** Sebelumnya hanya argumen URL pertama yang bertahan.
- **Reset menyelaraskan autostart sistem.** Entri peluncuran sistem kini mengikuti pengaturan yang tersimpan, tidak tetap aktif setelah reset.
- **Pemeriksaan follow yang berjalan tidak membatalkan perubahan baru.** Mengubah unduhan otomatis saat pemeriksaan tetap mempertahankan nilai baru.
- **Follow ganda terdeteksi lintas bentuk URL.** Perbedaan skema dan `www` tidak lagi membuat dua pemeriksaan untuk satu kanal.
- **Klik tray memulihkan jendela yang diminimalkan**, bukan menyembunyikannya.
- **Label tray mengikuti bahasa.** Mengubah bahasa kini menyusun ulang menu saat berjalan.
- **Jumlah impor dan pemulihan akurat.** Link yang digabung tidak lagi dihitung sebagai item baru.
- **Satu record rusak tidak menghapus seluruh file state.** Record riwayat, follow, dan pengaturan yang valid diselamatkan, berkas asli disimpan sebagai `.bad`.
- **Ekspor CSV bersifat atomik.** Kegagalan saat ekspor tidak meninggalkan file setengah jadi.
- **Edit aturan cepat diurutkan.** Mengubah dua aturan dengan cepat tidak lagi menggugurkan edit pertama.
- **Status bucket cookie jujur.** Toast sukses hanya muncul saat pengaturan benar-benar tersimpan.
- **Hotkey memilah link clipboard.** Menekan Ctrl+Shift+D dua kali tidak mengantrekan URL yang sama dua kali.
- **Semua link yang dijatuhkan masuk antrean.** Daftar URI dengan beberapa link tidak lagi kehilangan sisanya.
- **Label alat diterjemahkan saat berjalan.** Remux, encode ulang, dan original kini mengikuti bahasa yang dipilih.
- **Sakelar autostart mengikuti keadaan tersimpan.** Gagal menyimpan tidak lagi menampilkan sakelar sebagai aktif.
- **Error diagnostik terlihat.** Kegagalan muat ulang menghapus data basi, bukan membiarkan panel lama.
- **Label Inggris yang tersisa diterjemahkan.** Tagline, mesin siap, versi, identifier, dan lisensi memakai kunci lokal.
- **Menambah bucket tidak menghidupkan kembali bucket yang dihapus.** Edit bucket cepat diurutkan.

## v0.17.0

Profil per tautan, salin link antrean, dan audit native putaran berikutnya.

## Fitur baru

- **Profil situs.** Aturan unduhan kini bisa menimpa kualitas dan mode untuk link yang cocok, tidak hanya folder.
- **Salin URL sumber.** Setiap baris antrean punya tombol salin, jadi link bisa dipakai ulang tanpa membuka pustaka.

## Perbaikan bug

- **Jeda semua benar-benar menjeda.** Menjeda beberapa item tidak lagi membebaskan slot dan memulai unduhan yang menunggu.
- **Link tontonan tidak menarik seluruh playlist.** URL tontonan dengan `&list=` dianggap satu video kecuali entri playlist dipilih.
- **Unduhan ganda digabung.** Klik dua kali Download memakai item yang sudah aktif atau menunggu, bukan menulis file yang sama dua kali.
- **Unduhan subtitle masuk riwayat.** Pekerjaan subtitle saja kini mencatat entri selesai meski tidak menghasilkan file media.
- **Transfer tanpa ukuran tetap menampilkan byte.** Baris progres tanpa total kini memperbarui jumlah byte; file selesai melaporkan ukuran aslinya.
- **Error diklasifikasikan benar.** URL atau peringatan yang mengandung "404" atau "login" tidak lagi mengubah keputusan retry.
- **Error pratinjau menunjuk penyebabnya.** Baris `ERROR:` pertama mengalahkan peringatan di belakangnya.
- **Statistik saat buka tidak membengkak.** Item selesai yang sudah ada tidak dihitung sebagai selesai baru saat aplikasi dibuka.
- **Daftar bab mengikuti videonya.** Probes lambat tidak lagi memasang bab file lain ke video yang sedang diputar.
- **Batal memilih folder tidak bising.** Membatalkan dialog folder tidak lagi dilaporkan sebagai link tidak valid.
- **Teks status mengikuti bahasa.** Waktu di riwayat dan label status di Beranda mengikuti bahasa yang dipilih.
- **Lencana duplikat hanya untuk salinan ekstra.** File yang akan disimpan tidak diberi lencana duplikat.
- **Pencarian aman terhadap hasil basi.** Mengetik setelah analisis dimulai tidak membuat hasil lama mengulang pilihanmu.
- **Link tanpa skema menerima lebih banyak domain.** Deteksi host kini memakai TLD wajar apa pun, bukan daftar tetap.

## v0.16.0

Audit native putaran berikutnya, ambil link dengan seret, dan penulisan state yang lebih aman.

## Fitur baru

- **Jatuhkan link ke mana saja.** Seret URL dari browser ke Getta dan link langsung diambil tanpa dialog tambahan.

## Peningkatan

- **Toast lebih mudah diakses.** Region live selalu hadir, error memakai `alert`, dan tombol tutup diterjemahkan.
- **Dialog menerima fokus.** Onboarding dan dialog pemulihan memberi fokus diri dan tertutup dengan Escape.
- **Tanggal kedaluwarsa lokal.** Tanggal cookie memakai lokal perangkat, bukan UTC.

## Perbaikan bug

- **Pemulihan tetap tercatat.** Restore tidak menghapus entri recovery baru yang dibuat unduhan hasil restore.
- **Penulisan state diserial.** Unduhan bersamaan, ikutan, dan recovery tidak saling menimpa.
- **Gagal masuk ikutan tetap belum terlihat.** Link yang ditolak dicoba lagi pada pemeriksaan berikutnya.
- **Ikutan melihat lebih banyak unggahan.** Pemindaian playlist kini mencakup 200 entri per pemeriksaan.
- **Ekspor antrean mempertahankan opsi.** Format id, item playlist, dan folder tujuan ikut tersimpan.
- **Windows menyorot file.** Explorer menerima `/select,<path>` sebagai satu argumen.
- **Ekspor CSV tahan formula.** Sel yang diawali karakter formula dinetralisir.
- **Parameter alat berupa string.** Input angka tidak gagal saat menjalankan alat.
- **Alat menunggu FFmpeg.** Pemeriksaan mesin yang gagal menampilkan peringatan dan menonaktifkan Run.
- **Klik pengaturan tidak saling mengganggu.** Klik cepat pada aturan dan toggle tidak memakai state basi.
- **Onboarding melihat folder tersimpan.** Langkah 3 membaca output directory yang sudah ada.
- **Gagal mesin dan diagnostik terlihat.** Keduanya menampilkan tombol coba lagi.

## v0.15.0

Hasil audit mendalam dan dua kontrol unduhan baru.

## Fitur baru

- **Dependensi diperbarui.** Dua belas paket dan GitHub Actions naik ke versi baru.
- **Arsip unduhan.** Arsip ID opsional melewati link yang sudah diunduh, termasuk dalam playlist.
- **Coba ulang semua yang gagal.** Satu tombol memasukkan ulang semua item gagal dengan backoff dan pengaturan baru.

## Peningkatan

- **Waktu relatif lokal.** Waktu di pustaka dan ikutan mengikuti bahasa UI yang dipilih.
- **Statistik Beranda diperbarui.** Jumlah file dan byte tersimpan berubah saat Beranda tetap terbuka.
- **Pilihan palette direset.** Mengetik query baru menghapus seleksi keyboard yang lama.

## Perbaikan bug

- **PID basi dibersihkan.** Item selesai tidak menyimpan PID yang bisa membunuh proses lain saat dihapus.
- **Race saat lanjut diatasi.** Token generasi menghentikan job lama menimpa job pengganti.
- **Backoff retry dipertahankan.** Aktivitas antrean lain tidak bisa melewati tunggu setelah gagal rate-limit.
- **Playlist mencatat semua file.** Semua tujuan disimpan dan dihapus, bukan hanya file terakhir.
- **Cookie dan proxy berlaku di pratinjau.** Analisis dan pemeriksaan ikutan memakai konteks login yang sama dengan unduhan.
- **Instalasi mesin bersifat atomik.** Crash saat instalasi tidak meninggalkan yt-dlp yang terpotong.
- **Urutan antrean tetap benar.** Seret urutan menulis ke urutan yang ditampilkan UI.
- **Error Beranda terlihat.** Kegagalan memulai tidak menutup panel error.
- **Analisis lama diabaikan.** Pratinjau lambat tidak menimpa link yang lebih baru.
- **Pustaka tertutup dengan Escape.** Dialog mendengarkan window, bukan bergantung pada fokus.
- **Waktu relatif diterjemahkan.** String Inggris tidak muncul lagi di Bahasa Indonesia.

## v0.14.1

Progress, ETA, dan ukuran file muncul lagi saat mengunduh.

## Peningkatan

- **Catatan rilis sesuai standar.** Bagian Instalasi / Update kini ada dalam dua bahasa.
- **Nama installer diperbaiki.** Contoh di README cocok dengan nama aset rilis.

## Perbaikan bug

- **Progress dipaksa hidup.** yt-dlp diluncurkan dengan `--print`, yang diam-diam mematikan progress bar-nya. Getta kini mengirim `--progress`, sehingga persen, kecepatan, ETA, dan ukuran sampai ke UI secara real time.
- **Deteksi file dibuat eksplisit.** Path akhir kini berasal dari penanda `--print` berprefiks, sehingga file playlist dan hasil gabungan tercatat dengan benar.
- **Byte ditampilkan tanpa total.** Baris aktif menampilkan byte yang sudah terunduh meski situs tidak melaporkan ukuran total.

## v0.14.0

Konfigurasimu bisa dilihat dan dipindahkan.

## Fitur baru

- **Diagnostik di About.** Folder data, folder output, jumlah pustaka, channel diikuti, pemulihan tertunda, verifikasi mesin sekali klik, dan "Buka folder data".
- **Ekspor/impor antrean.** Simpan manifest ke file JSON dan muat ulang di komputer lain. Impor menerima maksimal 500 link per file.

## Perbaikan bug

- **Folder output kustom sekarang bisa diputar di pustaka.** File di luar Downloads dulu diblokir scope asset protocol. File yang dikenal kini ditambahkan ke scope saat unduhan selesai dan saat pustaka dimuat.
- **Pembersihan duplikat hanya menghapus file yang dikenal.** Perintah hapus kini memeriksa ulang path terhadap riwayat pustaka.

## v0.13.0

Atur urutan manifest sekarang.

## Fitur baru

- **Seret untuk menyusun antrean.** Item menunggu punya grip handle; seret untuk memindah posisinya. Bisa juga lewat keyboard: Tab ke grip, lalu tombol arah untuk memindah. Penyusunan ulang hanya berlaku untuk item menunggu.
- **Unduh lagi.** Pemutar pustaka kini menampilkan ukuran file dan tanggal, plus tombol "Unduh lagi" yang memasukkan item ke antrean dengan mode dan kualitas semula.
- **Item gagal ditandai merah.** Lencana antrean jadi merah saat ada item gagal.
- **Notifikasi lebih tenang.** Notifikasi selesai hanya terkirim saat jendela tidak fokus; toast di dalam aplikasi sudah konfirmasi.

## v0.12.0

Bekerja lebih cerdas: cari di antrean, pilih banyak item, urutkan pustaka.

## Fitur baru

- **Cari di antrean.** Filter sedang berjalan, menunggu, selesai, dan gagal berdasarkan judul, link, atau situs.
- **Urutkan pustaka.** Terbaru, terlama, nama, atau ukuran.
- **Pilih banyak.** Pilih entri di pustaka dan hapus sekaligus.
- **Hapus file duplikat.** Setelah memindai duplikat, hapus salinan berlebih dari disk (salinan pertama disimpan).
- **Periksa semua ikutan sekaligus** dari halaman Ikutan.

## Performa

- **Pekerjaan berat dipindah dari async runtime.** Pindai duplikat, cek integritas cookie, dan ffprobe chapters kini berjalan di blocking worker, jadi UI tetap gesit saat proses panjang.
- **Pengecekan ikutan diserialisasi.** Klik "periksa sekarang" berulang kali atau poll latar tidak bisa menambahkan unggahan yang sama dua kali.

## v0.11.2

Ctrl+Shift+D membaca clipboard secara native sekarang.

## Perbaikan bug

- **Ctrl+Shift+D mengantrekan dari clipboard.** Clipboard kini dibaca oleh backend Rust, bukan plugin frontend, jadi hotkey, tray, dan command palette membaca teks yang sama dengan error yang jelas kalau OS menolak.

## v0.11.1

Ctrl+Shift+D berfungsi lagi; toggle kurangi animasi benar-benar berfungsi.

## Perbaikan bug

- **Ctrl+Shift+D kembali mengantrekan dari clipboard.** Deteksi link kini menemukan URL di mana pun di clipboard: link Markdown, tanda kutip, tanda baca di akhir, karakter tak terlihat, dan host tanpa skema seperti `youtube.com/...` atau `youtu.be/...`. Clipboard yang tidak bisa dibaca dan "tidak ada link ditemukan" kini menghasilkan error yang berbeda.
- **Kurangi animasi sekarang bekerja.** Pengaturan mengatur atribut yang tidak dibaca CSS; animasi kini dimatikan saat aktif.

## v0.11.0

## Perbaikan bug

- **Crash tidak lagi membekukan antrean.** Mutex yang teracuni kini dipulihkan, bukan memicu panic di semua operasi berikutnya, jadi satu momen buruk tidak bisa menyetop semua unduhan.
- **Penulisan pengaturan yang berbarengan tidak lagi bertabrakan** pada file temp yang sama; setiap penulisan atomik memakai nama temp yang unik.
- **Parsing progress yt-dlp memakai ulang regex yang sudah terkompilasi**, tidak mengompilasi ulang per baris, menghemat CPU saat unduhan.

## Fitur baru

- **Menu tray "Unduh dari clipboard".** Tray mengantrekan link dari clipboard, menangani sama seperti hotkey global, lalu memunculkan jendela Getta.

## v0.10.2

## v0.10.2

Auto-update sekarang berfungsi benar di macOS.

## Perbaikan bug

- **Artifact updater macOS kini berbeda per arsitektur.** Build Apple Silicon dan Intel dikirim sebagai file terpisah, dan manifest updater menunjuk yang tepat untuk setiap Mac.

## v0.10.1

Perbaikan pipeline rilis; tidak ada perubahan aplikasi.

## Perbaikan bug

- **Pipeline penandatanganan diperbaiki.** Installer bertanda tangan dan manifest updater (`latest.json`) kini diproduksi dan dipublikasikan dalam satu tahap bersama rilisnya.

## v0.10.0

Getta sekarang bisa memperbarui dirinya sendiri.

## Fitur baru

- **Auto-update.** Halaman About menampilkan "Periksa pembaruan"; Getta mengunduh dan memasang rilis yang ditandatangani lalu restart. Di Linux berfungsi untuk AppImage.
- **Bucket cookie.** Simpan dan pakai ulang file cookies.txt per situs; ganti nama, periksa integritasnya secara lokal, atau pindahkan ke sampah dengan opsi pulihkan dan hapus permanen.
- **Chapter markers** di pemutar pustaka: lompat ke bab yang dibaca ffprobe.
- **Pindai duplikat** di pustaka: hash konten SHA-256, mengelompokkan duplikat, dan menghapus semua kecuali salinan pertama.
- **Pemeriksaan mesin** saat aplikasi dibuka: Getta memverifikasi yt-dlp dan FFmpeg bisa dijalankan dan memperingatkan kalau tidak.

## Peningkatan

- Dependensi diperbarui: keepawake 0.6.1, sanitize-filename 0.6.0, dirs 7.

## v0.9.0

Ikuti channel, aturan unduhan, fuzzy di mana-mana.

## Fitur baru

- **Ikuti channel.** Ikuti channel YouTube publik mana pun; Getta mendeteksi unggahan baru dan mengantrekannya. Opsional: cek latar tiap 6 jam dengan auto-unduh. Backlog yang sudah ada dilewati saat follow, jadi antrean tidak banjir.
- **Aturan unduhan.** Rute link ke folder berdasarkan pola, misalnya "youtube.com -> D:\Media\YouTube". Yang pertama cocok menang; folder yang dipilih manual selalu menang.
- **Command palette fuzzy.** Pencocokan subsequence dengan bonus batas kata, bukan sekadar substring.
- **Ekspor riwayat ke JSON** di samping opsi CSV.
- **Contact sheet**: sebarkan 20 frame sepanjang file ke dalam satu gambar.

## v0.8.0

Dua puluh dua alat di meja kerja, unduh subtitle saja, kecepatan pemutar.

## Fitur baru

- **Mode subtitle saja.** Unduh hanya aliran subtitle (SRT/VTT) tanpa media, dari pemilih mode di Beranda.
- **Enam alat baru:** gambar waveform, perkuat volume, potong sunyi, potong jadi kotak, gabung video + audio (ganti audio), sisipkan cover ke MP3.
- **Kecepatan putar** di pemutar inline: 0.5x sampai 3x.
- **Ekspor riwayat ke CSV** dari tab Riwayat.

## v0.7.0

Meja kerja bertumbuh dari 4 menjadi 16 alat, dikelompokkan per tipe.

## Fitur baru

- **Audio:** konversi ke MP3/FLAC/OGG/Opus/M4A/WAV, ekstrak aliran audio tanpa encode ulang, normalisasi volume ke -16 LUFS, percepat/perlambat pemutaran.
- **Video:** konversi container (remux tanpa encode ulang, atau encode ulang ke H.264/VP9), ubah ukuran ke 480p-2160p, putar, hapus audio, simpan satu frame sebagai JPG/PNG/WebP.
- **Gambar:** ubah ukuran ke lebar tetap, konversi ke JPG/PNG/WebP.
- **Subtitle:** konversi antara SRT, WebVTT, dan ASS.
- **Parameter alat bertipe.** Setiap alat punya parameter eksplisit; ekstensi file output masuk daftar putih.

## Perbaikan bug

- **In transit dan Antrean terpisah.** Item antre tidak lagi tampil di bagian In transit.
- **Antrean pulih sendiri.** Item antre atau yang menunggu retry langsung mulai begitu slot konkurensi kosong, tanpa menunggu pemicu eksternal.
- **Hitung mundur relay terlihat.** Baris antre menampilkan fase retry/backoff secara inline.
- **Progress di mana-mana.** Persen, byte terunduh / total, kecepatan, dan ETA tampil di baris aktif, plus strip agregat untuk seluruh antrean.

## v0.6.1

Nama baru untuk input beranda; tidak ada perubahan fungsi.

## Peningkatan

- **Getup.** Input di beranda kini bernama Getup di seluruh UI, dokumentasi, dan catatan rilis.

## v0.6.0

Ketik pencarian, potong klip, dan rapikan pustaka.

## Fitur baru

- **Pencarian YouTube di Getup.** Ketik pencarian biasa, bukan tautan; Getta menampilkan hasil teratas dan kamu pilih entri yang ingin diambil.
- **Potong video.** Menu Alat baru: potong klip (mulai + durasi dalam detik) dari file video atau audio apa pun.
- **Rapikan pustaka.** Setiap entri pustaka bisa menyalin URL sumbernya atau dihapus dari pustaka; file tetap di disk.

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

- **Getup di Beranda.** Satu kolom input menangani tautan tunggal, banyak tautan sekaligus, dan URL file langsung, dengan deteksi platform, pratinjau media, pemilih kualitas, serta mode audio/video saja.
- **Mesin unduhan.** Unduhan lokal berjalan lewat yt-dlp dan FFmpeg. Getta bisa memasang dan memverifikasi keduanya sendiri di Windows dan macOS; di Linux memakai paket dari distro.
- **Antrean dengan resume.** Antrean persisten dengan jeda, lanjutkan, batal, coba ulang dengan backoff, kecepatan live, ETA, dan label platform per situs. Jumlah unduhan bersamaan, jumlah koneksi, dan coba ulang bisa diatur.
- **Riwayat dan tampilkan.** Unduhan selesai terdaftar, bisa diunduh ulang, ditampilkan di file manager, atau dihapus.
- **UI dwibahasa.** Bahasa Inggris dan Bahasa Indonesia di semua layar, plus tema terang, gelap, dan sistem.
- **Tanpa akun, tanpa telemetri.** Getta tidak menghubungi server mana pun. Unduhan langsung dari situs ke diskmumu lewat mesin unduhan.
