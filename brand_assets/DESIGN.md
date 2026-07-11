# SkillBasin Tasarım Dili

> Tek doğruluk kaynağı: `src/index.css` içindeki CSS custom-property token'ları.
> Bu doküman o token'ların NEDEN'ini ve kullanım kurallarını saklar. Yeni UI eklerken
> önce burayı oku, HİÇBİR yerde ham hex/px kullanma — her zaman token.

## Kimlik

Estetik yön (Kerem, 2026-07-09): **Cursor + Vercel(Geist) + Claude.ai sentezi** —
"sade, kullanıcı dostu, şık, geliştiriciler için renkler iyi düşünülmüş".

Renk temaları (Kerem, 2026-07-10): iki ui.jln.dev vscode-theme portu, mod başına bir.
**Dark (VARSAYILAN) = "Aura Dark"**: menekşe-tonlu koyu yüzeyler (hue 249) + neon nane
aksan (#61ffca) — yüzey merdiveni kaynağın kendi kademelerinden (popover→app,
background→panel, muted→element, input→hover). **Light = "PowerShell ISE"**: sarımsı-
beyaz ISE kağıdı (hue 60) + ISE seçim mavisi (hue 213) + sarı vurgu.

- **Vercel'den**: disiplinli nötr taban, tek anlamlı aksan skalası, mono başlık/etiket
  + sans gövde eşlemesi, 6-14px köşe skalası, koyu zemin + belirgin border disiplini.
- **Aura'dan**: dark kimlik — menekşe zemin, neon nane tek aksan.
- **ISE'den**: light kimlik — editör kağıdı, koyu mavi vurgu, açık mavi seçim dolgusu.
- **Cursor'dan**: sessiz koyu chrome, katmanlı yüzeyler, kenar ışığıyla derinlik.

## Sabit kurallar

1. **Koyu tema varsayılandır** (`themePreference` başlangıcı `'dark'`). Her yeni
   bileşen ÖNCE dark'ta tasarlanır, light'a uyarlanır.
2. **Tek aksan**: `--accent-primary` — dark'ta Aura nanesi (#61ffca), light'ta ISE
   mavisi (#1c4e8c). İkinci bir marka rengi, gradient wordmark, rainbow YASAK. Stok
   Tailwind paletinden renk kopyalamak YASAK — her renk index.css token'ından gelir.
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
10. **Logo + wordmark (revizyon 2026-07-11, Kerem logosu)**: header'da 26px
   hexagon-havuz ikonu (`src/assets/logo-icon.png`; kaynak: `brand_assets/
   logo-icon.png` 1024px, `logo-full.png` yazılı tam logo — dış zemin şeffaf,
   hexagon içi de şeffaf: koyu temada app zeminiyle dolar). App ikon seti
   `npx tauri icon brand_assets/logo-icon.png` ile üretilir. Wordmark: mono, "Skill" `--text-primary` + "Basin" `--accent-primary`.

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
