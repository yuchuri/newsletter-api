# Newsletter API

一个基于 **Rust** 与 **Axum** 构建的生产级、高可靠邮件订阅与刊期发布平台。

本项目遵循 Clean Architecture 与类型驱动设计（Type-Driven Development）理念，解决了现代 Web 服务在面对高并发订阅、异步邮件投递、分布式会话管理以及重复提交等场景下的企业级核心难题。

---

## 目录
- [项目简介](#项目简介)
- [技术栈](#技术栈)
- [核心功能](#核心功能)
- [项目结构](#项目结构)
- [核心技术实现](#核心技术实现)
  - [1. 严格的幂等性设计 (Idempotency)](#1-严格的幂等性设计-idempotency)
  - [2. 事务性后台投递队列 (Transactional Worker)](#2-事务性后台投递队列-transactional-worker)
  - [3. 分布式 Session 与防时序攻击认证](#3-分布式-session-与防时序攻击认证)
  - [4. 领域驱动设计与强类型检验 (DDD)](#4-领域驱动设计与强类型检验-ddd)
  - [5. 编译期 SQL 验证与离线构建 (SQLx Offline)](#5-编译期-sql-验证与离线构建-sqlx-offline)
- [环境配置](#环境配置)
- [运行方式](#运行方式)
  - [方式一：Docker Compose 启动基础设施 + 本地启动应用（推荐开发）](#方式一docker-compose-启动基础设施--本地启动应用推荐开发)
  - [方式二：原生脚本启动基础设施](#方式二原生脚本启动基础设施)
  - [方式三：全容器化部署 (Docker)](#方式三全容器化部署-docker)
- [测试体系](#测试体系)
  - [运行测试](#运行测试)
  - [测试日志调试](#测试日志调试)
  - [生成代码测试覆盖率](#生成代码测试覆盖率)

---

## 项目简介

在现代 Web 平台中，构建一个可靠的邮件通知与订阅分发系统面临诸多挑战：网络抖动导致的重复发送、耗时邮件发送阻塞主请求、用户凭证安全与枚举漏洞、以及分布式环境下的状态同步问题。

`Newsletter API` 通过以下机制解决了上述问题：
1. **可靠的订阅流**：采用邮件双重验证（Double Opt-in）机制，防止恶意虚假邮箱注册。
2. **异步非阻塞分发**：将邮件发布与邮件投递彻底解耦，API 请求瞬间返回，由独立的后台 Worker 消费事务队列安全派发。
3. **零重复发送保证**：基于数据库唯一约束与响应持久化的幂等性设计，防止管理员重复提交发布相同刊期。
4. **全方位安全加固**：Argon2 密码哈希、基于 Redis 的 Signed Session 会话、防时序攻击（Timing Attack）的假哈希计算、以及 Secrecy 内存保护。

---

## 技术栈

| 领域 | 选型 | 说明 |
| :--- | :--- | :--- |
| **编程语言** | Rust (Edition 2024) | 内存安全、零成本抽象、极高并发性能 |
| **Web 框架** | [Axum](https://github.com/tokio-rs/axum) 0.8 | 基于 Tower/Hyper 的高性能、人体工程学异步 Web 框架 |
| **异步运行时** | [Tokio](https://tokio.rs/) 1.x | 多线程事件驱动的异步执行引擎 |
| **数据库** | PostgreSQL 18 + [SQLx](https://github.com/launchbadge/sqlx) 0.9 | 纯异步驱动、编译期 SQL 语法与类型安全检查 |
| **缓存与会话** | Redis 8 + `tower-sessions-redis-store` | 分布式 Session 存储与高并发状态支持 |
| **密码与加密** | Argon2 + Secrecy + Base64 | 密码抗 GPU 撞库哈希，密钥内存防泄露保护 |
| **模板引擎** | [Askama](https://github.com/djc/askama) | 类似 Jinja2 语法的编译期类型安全 HTML 渲染引擎 |
| **可观测性** | Tracing + Tracing-Subscriber | 结构化日志收集、UUIDv7 请求级全链路追踪 |
| **邮件服务** | Postmark REST API | 专业的交易类邮件发送服务（支持超时与重试） |
| **部署环境** | Azure Container Apps + ACR | Serverless 容器运行环境与弹性伸缩 |

---

## 核心功能

- [x] **邮件订阅 (Subscriptions)**：合法性校验（长度、字符、格式），存入数据库并处于 `unconfirmed` 状态。
- [x] **双重确认 (Double Opt-in)**：生成加密随机 Token 发送确认邮件，用户点击链接后完成 `confirmed` 状态流转。
- [x] **管理员认证与会话 (Admin Auth & Session)**：
  - 基于用户名与 Argon2 密码登录，登录状态持久化到 Redis。
  - 基于 Cookie 的签名防篡改机制。
  - 登录失败 Flash 消息通知。
- [x] **管理后台与密码修改 (Admin Dashboard & Password)**：
  - 受中间件保护的管理后台仪表盘。
  - 密码强校验与在线修改，登出时即时作废 Redis Session。
- [x] **幂等性刊期发布 (Idempotent Newsletter Delivery)**：
  - 支持 HTML / 纯文本双格式邮件正文。
  - 携带 `Idempotency-Key`，网络超时或重复点击不会二次重复投递。
- [x] **后台异步投递 Worker (Transactional Delivery Worker)**：
  - 刊期发布后原子写入 `issue_delivery_queue` 任务队列表。
  - 后台独立线程以分布式事务模式安全消费任务，逐个投递并在完成后出队。
- [x] **健康检查探针 (Health Check)**：提供 `/health_check` 供容器探针（Liveness/Readiness Probe）检测。

---

## 项目结构

```text
newsletter-api/
├── configuration/              # 环境配置文件
│   ├── base.yaml               # 基础默认配置
│   ├── local.yaml              # 本地开发配置
│   └── production.yaml         # 生产云端配置
├── crate/                      # 本地 Patch 的子模块
│   └── axum-messages/          # 兼容 Axum 0.8 的 Flash 消息处理层
├── migrations/                 # PostgreSQL 数据库迁移文件
│   ├── 20261021..._create_subscriptions_table.sql
│   ├── 20261111..._seed_user.sql
│   ├── 20261117..._create_issue_delivery_queue.sql
│   └── 20261119..._add_index_to_idempotency_created_at.sql
├── scripts/                    # 本地依赖初始化辅助脚本
│   ├── init_db.sh              # 启动 PostgreSQL 容器并自动执行迁移
│   └── init_redis.sh           # 启动本地 Redis 容器
├── src/                        # 业务源代码
│   ├── authentication/         # 密码哈希、校验中间件与登录认证
│   ├── domain/                 # 领域对象（SubscriberName, SubscriberEmail 等强类型验证）
│   ├── idempotency/            # 幂等性控制（Key 解析、状态流转、HTTP 响应持久化）
│   ├── routes/                 # HTTP API 与 HTML 页面路由处理器
│   │   ├── admin/              # 管理后台、密码修改、刊期发布
│   │   ├── login/              # 登录表单与提交
│   │   ├── health_check.rs     # 健康检查
│   │   ├── subscriptions.rs    # 订阅请求
│   │   └── subscriptions_confirm.rs # 订阅链接激活
│   ├── state/                  # 运行时共享状态（EmailClient, BaseUrl, DummyHash 等）
│   ├── configuration.rs        # 多源层次化配置读取（YAML + 环境变量）
│   ├── error.rs                # 统一错误处理与状态码映射 (AppError)
│   ├── issue_delivery_worker.rs# 后台事务邮件投递 Worker 循环
│   ├── main.rs                 # 应用程序入口（并发拉起 API 服务与后台 Worker）
│   ├── startup.rs              # Axum Application 结构封装与中间件装配
│   └── telemetry.rs            # Tracing 日志与链路初始化
├── templates/                  # Askama HTML 页面模板
├── tests/                      # 黑盒端到端集成测试套件
│   ├── common/                 # 测试脚手架（隔离数据库初始化、WireMock 邮件桩）
│   ├── admin_dashboard.rs      # 管理员仪表盘测试
│   ├── change_password.rs      # 密码修改流程测试
│   ├── health_check.rs         # 健康检查测试
│   ├── login.rs                # 登录与 Session 测试
│   ├── newsletter.rs           # 刊期发布、并发防重与瞬态错误测试
│   ├── subscriptions.rs        # 订阅逻辑集成测试
│   └── subscriptions_confirm.rs# 邮件链接激活确认测试
├── .sqlx/                      # SQLx 离线查询元数据缓存（支持无数据库 Docker 编译）
├── containerapp.yaml           # Azure Container Apps 部署拓扑模板
├── Dockerfile                  # 基于 cargo-chef 的轻量多阶段容器构建文件
└── docker-compose.yml          # 本地基础设施编排文件
```

---

## 核心技术实现

### 1. 严格的幂等性设计 (Idempotency)
为了避免用户重复点击“发送”导致同一篇刊期被投递多次，系统实现了类似分布式互斥锁的响应持久化机制：
- 客户端在提交刊期发布表单时携带由系统生成的随机唯一 `idempotency_key`。
- 请求进入时，开启数据库事务在 `idempotency` 表执行：
  ```sql
  INSERT INTO idempotency (user_id, idempotency_key)
  VALUES ($1, $2)
  ON CONFLICT DO NOTHING;
  ```
- **首次请求**：插入成功，返回数据库事务句柄继续执行业务；发布完成后，将生成的 HTTP 响应（状态码、响应头、正文）序列化存入该记录。
- **并发或重复请求**：由于唯一索引冲突导致插入 0 行，后续请求会检测到记录已存在，直接读取并返回之前保存的 HTTP 响应，杜绝底层任务被重复写入。

### 2. 事务性后台投递队列 (Transactional Worker)
邮件发布与邮件实际投递彻底解耦，防止因外部邮件服务延迟导致前端请求超时：
- 发布刊期时，在同一个数据库事务中原子写入 `newsletter_issues` 记录，并为当时所有的 confirmed 订阅者批量生成 `issue_delivery_queue` 记录。
- 独立的后台服务进程（`run_worker_until_stopped`）采用 **FOR UPDATE SKIP LOCKED** 机制出队任务：
  ```sql
  SELECT newsletter_issue_id, subscriber_email
  FROM issue_delivery_queue
  LIMIT 1
  FOR UPDATE
  SKIP LOCKED;
  ```
  保证多 Worker 实例部署时的并发安全与任务不丢失，发送成功后删除任务；出现瞬时故障时保留在队列中等待重试。

### 3. 分布式 Session 与防时序攻击认证
- **Session 持久化**：基于 `tower-sessions` 与 Redis，用户登录凭据以加密形式保存在客户端 Cookie 中，Session 数据与登录状态保存在 Redis，支持水平扩展。
- **防用户枚举的时序攻击防护 (DummyPasswordHash)**：
  在用户登录校验时，如果查询发现“用户不存在”，直接返回会导致处理时间明显短于“用户存在但密码比对失败”的情况，攻击者可通过统计接口响应时间枚举合法账号。
  本系统在内存中预先生成一个合法的 `DummyPasswordHash`，当用户名不存在时，依然对虚拟哈希执行耗时的 Argon2 校验，使得接口耗时保持一致，彻底杜绝侧信道泄露。

### 4. 领域驱动设计与强类型检验 (DDD)
采用 Rust 的类型系统实施**“解析优于验证”（Parse, don't validate）**原则：
- 使用 `SubscriberName` 与 `SubscriberEmail` 新类型（NewType），在构造阶段过滤非法字符、空串或畸形格式。
- 业务逻辑一旦拿到 `SubscriberEmail`，编译器即可保证其必为合法邮箱，无需在深层函数里到处进行防御式参数判断。

### 5. 编译期 SQL 验证与离线构建 (SQLx Offline)
- 所有 SQL 查询均通过 `sqlx::query!` 宏在编译期连接数据库进行语法、表结构以及字段类型与 Rust 类型的全双向静态检验。
- 配合 `.sqlx` 元数据缓存与 `ENV SQLX_OFFLINE=true`，在 Docker 容器打包与 CI/CD 流水线中**无需任何真实数据库连接**即可直接构建出纯净的独立二进制产物。

---

## 环境配置

项目通过分层配置管理加载环境变量（优先级：环境变量覆盖 > `production.yaml` / `local.yaml` > `base.yaml`）。

复制模版建立本地环境变量：
```bash
cp .env.example .env
```

| 环境变量名称 | 必填 | 默认值 / 示例 | 说明 |
| :--- | :---: | :--- | :--- |
| `APP_ENVIRONMENT` | 否 | `local` | 运行环境：`local` 或 `production` |
| `DATABASE_URL` | 是 | `postgres://postgres:password@localhost:5432/newsletter` | 本地 PostgreSQL 连接串（sqlx 编译使用） |
| `HMAC_SECRET` | 是 | `<随机Base64字符串>` | 64字节 Base64 编码密钥，用于 Session Cookie 签名 |
| `APP_DATABASE__HOST` | 否 | `127.0.0.1` | 数据库主机名 |
| `APP_DATABASE__PORT` | 否 | `5432` | 数据库端口 |
| `APP_DATABASE__USERNAME`| 否 | `postgres` | 数据库用户名 |
| `APP_DATABASE__PASSWORD`| 否 | `password` | 数据库密码 |
| `APP_DATABASE__DATABASE_NAME`| 否 | `newsletter` | 业务数据库名称 |
| `APP_REDIS_URI` | 否 | `redis://127.0.0.1:6379` | Redis 连接串 |
| `APP_APPLICATION__PORT`| 否 | `8000` | HTTP 服务监听端口 |
| `APP_APPLICATION__BASE_URL` | 否 | `http://127.0.0.1:8000` | 应用公网域名（生成邮件确认链接用） |
| `APP_EMAIL_CLIENT__AUTHORIZATION_TOKEN` | 否 | `my-secret-token` | Postmark Server API Token |
| `APP_EMAIL_CLIENT__SENDER_EMAIL` | 否 | `test@gmail.com` | 已验证的发信人邮箱 |

---

## 运行方式

### 方式一：Docker Compose 启动基础设施 + 本地启动应用（推荐开发）

本项目提供了配套的 `docker-compose.yml`，可一键在后台拉起 PostgreSQL 18 与 Redis 8：

```bash
# 1. 启动本地 Postgres 与 Redis
docker compose up -d

# 2. 安装 sqlx-cli（若尚未安装）
cargo install sqlx-cli --no-default-features --features rustls,postgres

# 3. 执行数据库迁移，初始化表结构与种子数据
cargo sqlx migrate run

# 4. 启动后端应用程序（API 服务与 Worker 进程将同时启动）
cargo run
```

服务就绪后，访问：
- **首页**：[http://127.0.0.1:8000](http://127.0.0.1:8000)
- **健康检查**：[http://127.0.0.1:8000/health_check](http://127.0.0.1:8000/health_check)
- **管理后台登录**：[http://127.0.0.1:8000/login](http://127.0.0.1:8000/login)  
  *(默认管理员账号：`admin` / 密码：`admin`，由种子迁移文件注入)*

---

### 方式二：原生脚本启动基础设施

如果不想使用 docker compose，也可以使用项目内置的脚本分别启动依赖容器：

```bash
# 启动 PostgreSQL 并自动执行全部数据迁移
./scripts/init_db.sh

# 启动 Redis 容器
./scripts/init_redis.sh

# 启动应用程序
cargo run
```

---

### 方式三：全容器化部署 (Docker)

项目采用 `cargo-chef` 实现多阶段构建，极大节省编译时间，并内置了 Redis 守护进程以满足单容器环境下的零配置启动：

```bash
# 构建生产级 Docker 镜像
docker build -t newsletter-api:latest .

# 运行镜像
docker run -p 8000:8000 \
  -e APP_ENVIRONMENT=production \
  -e APP_APPLICATION__BASE_URL=http://localhost:8000 \
  -e HMAC_SECRET="$(openssl rand -base64 64)" \
  newsletter-api:latest
```

---

## 测试体系

项目拥有非常完备的**黑盒端到端集成测试**套件（位于 `tests/` 目录）。为了保证测试之间互不干扰：
- 每个测试用例在启动时，都会**自动创建一个带随机 UUID 的全新独立数据库**。
- 自动在该独立数据库中运行全部迁移。
- 使用 `wiremock` 在后台起一个独立的 HTTP Mock 服务器模拟真实邮件投递 API，无需联网即可测试完整邮件收发闭环。
- 测试完成后自动断开并销毁临时数据库。

### 运行测试

```bash
# 运行全部单元测试与集成测试
cargo test
```

### 测试日志调试

默认情况下，集成测试会抑制标准输出以保持控制台整洁。如果你想查看某项测试在执行时的详细 `tracing` 诊断日志，带上 `TEST_LOG=true` 环境变量即可：

```bash
TEST_LOG=true cargo test health_check_works
```

### 生成代码测试覆盖率

项目支持使用 `cargo-llvm-cov` 生成高精度的源码级覆盖率报告：

```bash
# 安装覆盖率工具
cargo install cargo-llvm-cov

# 运行测试并在浏览器中打开 HTML 覆盖率报告
cargo llvm-cov --html --open
```

---

## 许可证
本项目代码仅供学习与工程实践参考。
