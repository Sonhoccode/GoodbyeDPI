# GoodbyeDPI Manager by Sonhoccode

Ứng dụng Windows do **[Sonhoccode](https://github.com/Sonhoccode)** xây dựng để cài đặt và quản lý GoodbyeDPI bằng giao diện. Đây là bản fork cộng đồng của [GoodbyeDPI gốc bởi ValdikSS](https://github.com/ValdikSS/GoodbyeDPI), không phải bản phát hành chính thức của tác giả gốc.

## Tải và cài đặt

Tải bản mới nhất tại **[Releases](https://github.com/Sonhoccode/GoodbyeDPI/releases/latest)**. Với bản đầu tiên: [GoodbyeDPI Manager 1.00](https://github.com/Sonhoccode/GoodbyeDPI/releases/tag/v1.00).

Chạy file `GoodbyeDPI-Manager-All-in-One-1.00.exe` trên Windows 10/11 x64 và chấp nhận yêu cầu Administrator. Bộ cài kèm GoodbyeDPI 0.2.2, WinDivert và ứng dụng quản lý. Nếu máy đã có dịch vụ `GoodbyeDPI`, bộ cài không ghi đè dịch vụ đó.

## Chức năng

- Bật/tắt và xem trạng thái dịch vụ GoodbyeDPI.
- Chọn preset `-5` hoặc `-6`; cần dừng dịch vụ trước khi áp dụng.
- Chọn dịch vụ tự khởi động cùng Windows.
- Giao diện tiếng Anh (mặc định), tiếng Việt, tiếng Tây Ban Nha và tiếng Nga; ghi nhớ ngôn ngữ đã chọn.
- Tự kiểm tra release mới khi mở ứng dụng. Khi có bản mới, người dùng chọn **Install** để tải, kiểm tra SHA-256 và chạy bộ cài. Việc cập nhật không diễn ra âm thầm.
- Gỡ cài đặt từ trang Settings.

GoodbyeDPI có thể không hoạt động trên mọi mạng. Ứng dụng cần quyền quản trị để thay đổi dịch vụ Windows. Bản phát hành chưa có chữ ký số thương mại.

## Buy me a coffee ☕

Nếu ứng dụng hữu ích, bạn có thể ủng hộ mình qua mã VietQR dưới đây. Đây là khoản ủng hộ tự nguyện, không ảnh hưởng đến chức năng ứng dụng.

<img src="manager/assets/coffee.jpg" alt="Mã VietQR ủng hộ Sonhoccode" width="320">

Mã QR cũng có thể mở bằng nút **Buy me a coffee** trong ứng dụng.

## Mã nguồn và ghi công

Mã Rust của ứng dụng, icon và bộ cài nằm trong [`manager/`](manager/). Hướng dẫn build: [`manager/README.md`](manager/README.md).

GoodbyeDPI engine thuộc [ValdikSS](https://github.com/ValdikSS/GoodbyeDPI); WinDivert thuộc [basil00](https://github.com/basil00/Divert). Các giấy phép đi kèm trong [`manager/engine/licenses/`](manager/engine/licenses/). Mã nguồn engine gốc trong fork này vẫn giữ nguyên lịch sử và giấy phép upstream.
