# مهسا دسک

ارئه‌ی دسکتاپ [مهسانگ](https://github.com/GFW-knocker/MahsaNG) (v2rayNG fork) برای ویندوز. برنامه گپ-source و غیرانتفاعی که کاربران می‌توانند کانفیگ‌های خود را منتشر کنند تا دیگران بتوانند به اینترنت آزاد دسترسی داشته باشند.

![مهسا دسک](icons/icon.ico)

## 📋 مرور

مهسا دسک یک برنامه دسکتاپ ویندوز است که رابط کاربری کاربردی برای مدیریت کانفیگ‌های پراکسی V2Ray/Xray فراهم می‌کند. این برنامه امکانات زیر را ارائه می‌دهد:

- اتصال به سرورهای پروکسی با هسته Xray
- مدیریت لینک‌های اشتراک برای دریافت خودکار کانفیگ سرورها
- تنظیم پیشرفتهRouting, DNS وrypto تنظیمات
- استفاده از تونل Psiphon در کنار کانفیگ‌های پروکسی
- انجامdiagnostics شبکه و آزمون سرعت
- سفارشی‌سازی حالت پروکسی سیستم و VPN TUN

 برنامه به صورت نصب‌کننده NSIS و قابل portar ارائه می‌شود.

## ✨ امکانات

### امکانات اصلی (نسخه 0.3.2)

- **کره Xray** (ریزشیپ رسمی XTLS) که توسط برنامه مدیریت می‌شود - SOCKS 10808 / HTTP 10809
- **حالت_proxy سیستم** - سیستم_proxy ویندوز را در هنگام اتصال/قطع خودکار می‌کند
- **حالت TUN (آزمایشی)** - VPN کامل سیستم از طریق sing-box + wintun همراه، نیاز به aprovat(UAC) دارد
- **پشتیبانی Psiphon** - اجرای psiphon-tunnel-core.exe به عنوان تونل aslone ("only" mode) یا زنجیره روی کانفیگ فعّال ("after" mode)
- **وارد کردن لینک‌های اشتراک**: vmess/vless (REALITY, XTLS-vision)، trojan، shadowsocks، socks، http، hysteria2 و بارهایpayload base64
- **ابزار QR**: کدنود QR codes از تصویر clipboard، گرفتن ناحیه صفحه و وارد کردن کانفیگ، نشان دادنProfiles as QR codes
- **ابزارهای تست**: TCP ping، آزمون واژهPager latency در هسته వేامی instance (فردی و دسته‌ای)، ابزار اعتبارسنجی کانفیگ
- **Routing per-app (TUN)**: انتخاب برنامه‌هایی که از تونل بypass می‌شوند یا تنها آن‌ها از آن عبور می‌کنند
- **اشتراک‌ها** - افزودن URLs با به‌روزرسانی خودکار زمان‌بندی شده
- **تنظیمات پارتی TLS** (بسته‌ها/طول/فاصله) پیچیده به dialer TLS Xray
- **بازنویسی IP CDN** - آدرس اتصال را با IP Cloudflare جایگزین کنید در حالی که SNI/Host preserved
- **وابستگی‌های маршруتی** - پیش‌فرض proxy همه / دور زدن ایران (+چین) با بلاک تبلیغات، اضافه برomain per-domain overrides
- **مدیریت داده جیو** - مشاهده، به‌روزرسانی و حذف geoip.dat/geosite.dat
- **تنظیمات DNS** (remote DoH/UDP)، شنود، MUX، allow-insecure، اولویت IPv6
- **سیستم سبر** - اتصال/قطع، باز کردن، خروج؛ کمینه کردن به سبر، راه‌اندازی خودکار در/Login؛ خودistent در startup
- **گزارش‌های زنده هسته** و ліقounters حركة ترافیک
- **پشتیبانی Hysteria2** - parsing hysteria2:// / hy2:// لینک‌ها؛ اجرای örtnative از طریق موتور TUN
- **وارد کردن لینک عمیق** - `mahsadesk://` URL scheme برای truyền کانفیگ به برنامه
- **راند و انگلیسی** رابط کاربری با پشتیبانی از جهت راست‌به‌گ left (RTL) و تعویض خودکار
- **صادر/وارد کانفیگ** کامل (سرورها + subs + settings) به صورت JSON

### جUpdates در نسخه 0.3.2

- **بررسی به‌روزرسانی در startup** - نسخه را بدونگ silently با feed انتشار GitHub مقایسه می‌کند؛ با ظاهر bandozi که لینک مستقیم نصب گشاد می‌شود وقتی نسخه جدید موجود است (EN + FA)
- **رنگ‌بندی سبز/قرمز переключатели** - همه کلیدها روشن/خودکار و نقطه状况 اتصال از رنگ‌های thươngمfixed (#01ca7a on / #f03b4c off) در هر دو پوسته استفاده می‌کنند

### جUpdates در نسخه 0.3.1

- **انتخاب پوسته روشن/تاریک/خودکار**؛ `auto` دنبال رنگ سیستم می‌گردد
- **مدیریت نیازمندی‌ها** - هر_kernel дра Engine (Xray، sing-box، wintun، geo data، Psiphon engine) را مقابل سرورهای عليا بررسی می‌کند، نسخه‌ها را نشان می‌دهد و آن‌ها را در جا.place به‌روز می‌کند
- **ساخت dựng کامل** - نصب‌کننده و فایل zip قابل portar همه runtime را نیاز می‌بundi، شامل `psiphon-tunnel-core.exe`

### آنچه در dışی است (به قصد طراحی)

مهساNG سرویس کانفیگ‌های رایگانgifted وابسته به بخش‌های بسته‌ kode (رمزنگاری و احراز هویت mahsaserver) است که در این نسخه دسکتاپ بازسازی نشده. کانفیگ‌های شخصی یا هر اشتراک عمومی را استفاده کنید.

## 🛠️ ساخت از منبع

```powershell
cd MahsaDesk
npm install
# fetch xray.exe / sing-box.exe / wintun.dll / geo files into src-tauri/binaries
powershell -NoProfile -ExecutionPolicy Bypass -File .scripts/download-binaries.ps1
npm run tauri build
```

نصب‌کننده NSIS در `src-tauri/target/release/bundle/nsis/` قرار می‌گیرد؛ исполняемый файл portabl `src-tauri/target/release/MahsaDesk.exe` ( پوشه binaries рядом با آن نگه دارید).

CI builds و انتشار releases خودکار روی `v*` tags.

## 🔒 حریم خصوصی

برنامه تنها با سرورهای پروکسی شما و لینک‌های اشتراک شما صحبت می‌کند. هیچanalytics، هیچtelemetry. تنظیمات در `%APPDATA%\com.mahsadesk.desktop` ذخیره می‌شود.

## 📦 پروژه‌های مرجع

 پروژه‌های متن‌باز زیر الهام‌بخش و پایه این پروژه هستند:

- **[MahsaNG](https://github.com/GFW-knocker/MahsaNG)** - Android v2rayNG fork که作为 پایه این версия دسکتاپ عمل کرده
- **[v2rayNG](https://github.com/2dust/v2rayNG)** - Android V2Ray client اصلی
- **[Xray-core](https://github.com/XTLS/Xray-core)** - موتور_proxy اصلی استفاده شده توسط MahsaDesk
- **[sing-box](https://github.com/sing-box/sing-box)** - پلتفرم_proxy सार्व visor برای حالت TUN
- **[psiphon-tunnel-core](https://github.com/Psiphon-Inc/psiphon-tunnel-core)** - موتور تونل Psiphon که در MahsaDesk یکپارچه شده
- **[@tauri-apps](https://tauri.app)** - فریم‌ورک استفاده شده برای این برنامه دسکتاپ

## 🌐 پشتیبانی از زبان

- **انگلیسی** (پیش‌فرض)
- **فارسی** - پشتیبانی کامل RTL با ترجمه کامل رابط کاربری

## 📄 مجوز

این پروژه متن‌باز است و تحت مجوز MIT منتشر شده است.