# SkillBasin Tasarım Dili

> Tek doğruluk kaynağı: `src/index.css` içindeki CSS custom-property token'ları.
> Bu doküman o token'ların NEDEN'ini ve kullanım kurallarını saklar. Yeni UI eklerken
> önce burayı oku, HİÇBİR yerde ham hex/px kullanma — her zaman token.

## Kimlik

Estetik yön (Kerem, 2026-07-09): **Cursor + Vercel(Geist) + Claude.ai sentezi** —
"sade, kullanıcı dostu, şık, geliştiriciler için renkler iyi düşünülmüş".

- **Vercel'den**: disiplinli nötr taban, tek anlamlı aksan skalası, mono başlık/etiket
  + sans gövde eşlemesi, 6-14px köşe skalası, near-black dark + belirgin ~#333 border.
- **Claude.ai'den**: light temanın sıcak kağıt tonu (#f6f5f1 — soğuk SaaS beyazı değil).
- **Cursor'dan**: sessiz koyu chrome, katmanlı yüzeyler, kenar ışığıyla derinlik.

Karar kayıtları: docs/DECISIONS.md D12 (koyu varsayılan, ikonsuz mono wordmark,
dark kontrast revizyonu).

## Sabit kurallar

1. **Koyu tema varsayılandır** (`themePreference` başlangıcı `'dark'`). Her yeni
   bileşen ÖNCE dark'ta tasarlanır, light'a uyarlanır.
2. **Tek aksan**: basin teal (`--accent-primary`). İkinci bir marka rengi, gradient
   wordmark, rainbow YASAK. Stok Tailwind mavi/indigo YASAK.
3. **Yüzey hiyerarşisi**: `bg-app → bg-panel → bg-element → bg-element-hover`
   kademeleri atlanmaz; panel üstüne panel koyma. Dark'ta ayrışma border'dan gelir
   (`border-subtle` her zaman üstünde durduğu yüzeyden net açık).
4. **Gölge**: yalnız `--shadow-sm` / `--shadow-lg`. Dark gölgeleri üst kenar iç-ışığı
   içerir (saf siyah gölge koyu zeminde görünmez) — elle gölge yazma.
5. **Tipografi**: veri-biçimli her şey mono (`--font-mono`: Fira Code — versiyonlar,
   path'ler, sayaçlar, wordmark); düzyazı sans (`--font-ui`: Fira Sans). Başlık ve
   gövde aynı ağırlıkta bırakılmaz.
6. **Köşe**: `--radius-sm` 6px (input/buton), `--radius-md` 10px (kart içi eleman),
   `--radius-lg` 14px (panel/kart), `--radius-pill` (chip/pill/CTA).
7. **Animasyon**: yalnız `transform` + `opacity`. `transition-all` YASAK
   (App.css'te kalan eski kullanımlar temizlenecek borç — yenisini ekleme).
8. **Etkileşim**: her tıklanabilir elemanda hover + focus-visible + active. Ghost
   ikon butonlar (`.card-btn`) hover'da yüzey + border kazanır.
9. **i18n**: her yeni UI string'i en + tr + zh üçüne birden eklenir
   (`src/i18n/resources.ts`), sadece en'e bırakılmaz. Sayılı string'lerde çoğul
   formu `_one/_other` ile ver ("1 files" hatası tekrarlanmasın).
10. **Wordmark**: ikonsuz, mono, "Skill" `--text-primary` + "Basin"
    `--accent-primary`. Logo görseli geri getirilmez.

## Bileşen desenleri (App.css'te hazır — yenisini uydurmadan bunları kullan)

- `.chip` / `.chip-accent` / `.chip-pin` — küçük durum etiketleri
- `.tool-pill` (`.active/.inactive/.disabled`) + `.tool-pill-version` — araç rozetleri
- `.settings-toggle` + `.settings-toggle-knob` (knob rengi `--toggle-knob-bg`)
- `.btn.btn-primary` / `.btn.btn-secondary`, `.icon-btn`, `.card-btn`
- `.versions-panel` / `.versions-box` / `.pin-matrix*`
- Onay akışları: in-app modal (bkz. `pendingSharedToggle` deseni) —
  `window.confirm/prompt/alert` YASAK (WebView2'de güvenilmez)

## Doğrulama döngüsü

UI değişikliğinde frontend-quality skill'inin screenshot döngüsü: vite dev server
(5173) + tarayıcı screenshot, dark ÖNCE, min 2 tur. Tauri'ye özgü davranışlar için
CDP yolu: `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9224` ile
`npm run tauri:dev`, puppeteer-core `browserURL` bağlantısı.
