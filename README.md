# csustkit-rs

csustkit-rs 是一个访问长沙理工大学校园服务的 Rust 库，旨在为 iOS、Android 等客户端提供可复用的网络层。

它支持直连和 WebVPN，覆盖统一身份认证、教务系统、校园卡、网络课程中心、超星学习通和大学物理实验平台，可用于查询课表、成绩、考试、作业和宿舍电量等信息。

## 快速体验

安装 Rust 工具链后，在仓库根目录运行 Rust 交互示例。

```sh
cargo run --locked --example csustkit
```

进入「校园服务」后，选择校园网直连或 WebVPN，按提示完成统一认证，再选择要查询的服务。大物实验使用独立账号，也可以从主菜单直接进入。WebVPN 链接转换无需登录。

账号和密码可以在运行时输入，也可以将下面的变量设为环境变量，或写入仓库根目录的 `.env` 文件。

```dotenv
CSUST_AUTHSERVER_USERNAME=统一认证用户名
CSUST_AUTHSERVER_PASSWORD=统一认证密码
CSUST_PHYSICS_EXPERIMENT_USERNAME=大物实验用户名
CSUST_PHYSICS_EXPERIMENT_PASSWORD=大物实验密码
```

Swift 和 Kotlin 也提供相同的交互示例，以下命令均在仓库根目录执行。

- **Swift** 示例需要 Apple 芯片的 Mac 和 Xcode。先用 `rustup target add aarch64-apple-darwin aarch64-apple-ios aarch64-apple-ios-sim` 安装目标，再运行 `./tools/build-swift-package.sh` 和 `swift run --package-path examples/swift csustkit-swift`。
- **Kotlin** 示例需要 JDK 17。运行 `./android/gradlew -p examples/kotlin run`，Gradle 会编译 Rust 库并生成 Kotlin 绑定。

## 接入项目

- **Rust** 项目在 `Cargo.toml` 的 `[dependencies]` 中添加 `csustkit = { path = "../csustkit-rs" }`，按目录结构调整路径。创建 `CsustSession`，选定 `ConnectionMode`，使用 `SsoHelper` 完成统一认证。以教务为例，调用 `login_to_education()` 后，用相同的连接模式和会话创建 `EduHelper`，即可查询课表等数据。完整流程见 [Rust 示例](examples/csustkit/main.rs)。
- **iOS / macOS** 项目运行上面的 Swift 构建脚本后，将仓库作为本地 Swift Package 添加到项目，引入 `CSUSTKitRust`。支持 iOS 17+ 和 Apple 芯片的 macOS 14+。
- **Android** 项目需要 JDK 17、Android SDK 36、NDK `28.2.13676358`、`cargo-ndk` 和 Rust 目标 `aarch64-linux-android` 来构建库。将 `ANDROID_HOME` 指向 SDK 目录后，运行 `./android/gradlew -p android publishReleasePublicationToDistributionRepository`。将生成的 `android/build/maven` 添加为 Gradle 的 Maven 仓库，并引入 `com.zhelearn:csustkit-android:0.1.0`。运行环境要求 `arm64-v8a` 和 API 26+。创建会话前先调用 `CsustKitAndroid.initialize(context)`。
