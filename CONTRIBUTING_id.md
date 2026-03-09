# Panduan Berkontribusi ke Aptos Core 🇮🇩

> 📌 Ini adalah terjemahan tidak resmi `CONTRIBUTING.md` ke Bahasa Indonesia.  
> Dokumen asli tersedia di [CONTRIBUTING.md](./CONTRIBUTING.md).  
> Apabila ada perbedaan antara versi ini dan versi asli, versi asli yang berlaku.

---

## Tujuan Kami

Tujuan kami adalah membuat proses kontribusi ke Aptos Core semudah dan setransparan mungkin. Lihat [Komunitas Aptos](https://aptosfoundation.org/) untuk detail lengkap.

---

## Proses Pengembangan

Halaman ini menjelaskan proses pengembangan kami.

### Memulai Kontribusi

Untuk berkontribusi pada implementasi Aptos Core, mulailah dengan menyiapkan salinan pengembangan yang benar:

1. Gunakan antarmuka GitHub untuk **fork** repositori `aptos-core`
2. **Clone** hasil fork Anda ke mesin lokal:
   ```bash
   git clone https://github.com/USERNAME_ANDA/aptos-core.git
   cd aptos-core
   ```
3. Untuk setup lingkungan pengembangan dan build pertama, lihat panduan [Building Aptos From Source](https://aptos.dev/nodes/building-from-source)

---

## Panduan Penulisan Kode

- Ikuti **[Panduan Penulisan Kode](./RUST_CODING_STYLE.md)** untuk bahasa pemrograman Move dan Rust
- Pastikan Anda juga mengikuti **[Panduan Kode Aman](./RUST_SECURE_CODING.md)** untuk berkontribusi secara aman ke Aptos

---

## Kontribusi Dokumentasi

Website developer Aptos Core juga bersifat open source (kodenya tersedia di repositori ini). Website ini dibangun menggunakan [Docusaurus](https://docusaurus.io/). Jika Anda sudah mengenal Markdown, Anda sudah bisa berkontribusi!

---

## Pull Request (PR)

Perubahan pada proyek diajukan melalui **Pull Request**. Berikut panduannya:

### Langkah-langkah Membuat PR

1. **Fork** repositori ini
2. Buat **branch baru** dari branch `main`:
   ```bash
   git checkout -b nama-fitur-atau-perbaikan-anda
   ```
3. Lakukan perubahan Anda
4. **Commit** perubahan dengan pesan yang jelas:
   ```bash
   git commit -m "tipe: deskripsi singkat perubahan"
   ```
5. **Push** ke fork Anda:
   ```bash
   git push origin nama-fitur-atau-perbaikan-anda
   ```
6. Buka **Pull Request** ke repositori utama melalui GitHub

### Format Pesan Commit

Gunakan format berikut untuk pesan commit:

| Tipe | Kapan Digunakan |
|------|----------------|
| `feat` | Menambahkan fitur baru |
| `fix` | Memperbaiki bug |
| `docs` | Perubahan dokumentasi |
| `refactor` | Refactoring kode |
| `test` | Menambah atau memperbaiki test |
| `chore` | Perubahan konfigurasi/build |

**Contoh:**
```
docs: tambah terjemahan CONTRIBUTING.md ke Bahasa Indonesia
fix: perbaiki panic saat memproses transaksi kosong
feat: tambah dukungan multi-signature pada wallet adapter
```

---

## Melaporkan Bug

Aptos Core menggunakan **GitHub Issues** untuk melacak bug.

### Sebelum Membuat Issue Baru

- Periksa apakah bug sudah pernah dilaporkan sebelumnya
- Cari di daftar [Issues yang ada](https://github.com/aptos-labs/aptos-core/issues)

### Cara Melaporkan Bug yang Baik

Sertakan informasi berikut dalam laporan bug Anda:

1. **Deskripsi singkat** — Apa yang terjadi?
2. **Langkah reproduksi** — Bagaimana cara mengulang bug tersebut?
3. **Perilaku yang diharapkan** — Apa yang seharusnya terjadi?
4. **Perilaku aktual** — Apa yang sebenarnya terjadi?
5. **Lingkungan** — Versi OS, Rust, dan Aptos yang digunakan
6. **Screenshot atau log** — Jika memungkinkan

---

## Lisensi

Dengan berkontribusi ke Aptos Core, Anda setuju bahwa kontribusi Anda akan dilisensikan di bawah **[Innovation-Enabling Source Code License](./LICENSE)**.

---

## Bergabung dengan Komunitas

Ada pertanyaan atau butuh bantuan? Bergabunglah dengan komunitas Aptos:

- 🐦 [Twitter/X Aptos](https://twitter.com/Aptos)
- 💬 [Discord Aptos](https://discord.gg/aptosnetwork)
- 🌐 [Website Developer](https://aptos.dev)
- 📖 [Forum Developer](https://github.com/aptos-labs/aptos-developer-discussions/discussions)

---

*Terjemahan ini dibuat oleh kontributor komunitas untuk memudahkan developer Indonesia dalam berkontribusi ke ekosistem Aptos.*
