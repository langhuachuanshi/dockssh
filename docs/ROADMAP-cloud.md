# DockSSH 云端密钥 + 配置同步 商业化计划

> 版本:v0.1 草案 · 日期:2026-06-19 · 状态:**方向规划,暂不实施**
> 当前项目阶段:**基础功能开发**(本地 SSH/Docker 管理),本文档为中长期路线图。
> 推进方式:按阶段一步步来,每个需要拍板的关键点都会单独询问 + 解释为什么这么做。

## 0. 定位与核心结论

- **产品定位**:对标 Termius / OrcaTerm 的个人 SSH 管理工具,freemium 商业模式。
- **付费核心价值**:云同步 + 多设备(行业验证过的转化点)。
- **技术选型结论**:自建 Go 后端(Docker 部署),**端到端加密**(服务端是哑管道),不用 CF/EdgeOne/Supabase——已有服务器和主后端,自建最优。
- **节奏结论**:**现在不做**。先把本地基础功能做扎实,云同步作为 v2 重点功能启动。

---

## 1. 整体架构(未来形态)

```
┌─────────────────────────────────────────────────┐
│ dockssh 客户端 (Tauri/Rust)                      │
│  本地明文(内存)  ←─ 解密 ──  本地密文(磁盘)    │
│  Hosts/DEK        ── 加密 ──→ vault.local.json   │
│        │加密层(Argon2id+AES-GCM)│同步层(reqwest)│
└────────┼──────────────────────────┼─────────────┘
         │ HTTPS + JWT              │ 密文上传
         ▼                          ▼
┌─────────────────────┐   ┌──────────────────────┐
│ 主后端(已有)        │   │ vault-service(Go,新) │
│ users / auth         │   │ /setup /unlock       │
│ subscriptions ★      │   │ /items /push /recover│
│ 支付回调 ★           │   │ billing: 配额拦截 ★  │
│ 发 JWT(带 plan)      │   │ 哑管道:只存取密文   │
└─────────┬────────────┘   └──────────┬───────────┘
          │                           │
          └─────────┬─────────────────┘
                    ▼
              Postgres(共享)
              vault_users / vault_items
              / subscriptions
```

**两个新服务容器**:`vault-service`(同步后端)、复用现有 Postgres。主后端只多一张 `subscriptions` 表 + 支付回调。

---

## 2. 安全模型(密钥学,核心)

照 Bitwarden 零知识 / 两层密钥模型:

| 密钥 | 来源 | 作用 | 离开设备? |
|------|------|------|----------|
| Master Password | 用户输入 | 派生 KEK | 否 |
| KEK | Argon2id(pw, salt) | 解密 DEK | 否(永不) |
| DEK | 注册时随机生成 | 加密每条 vault 项 | 仅密文(`protected_dek`)上传 |
| Ed25519(可选,二期) | 注册时生成 | 防服务端篡改 | 仅公钥上传 |

- 算法:Argon2id(64MiB/3/4) + AES-256-GCM + Ed25519 + zeroize
- 服务端**永远不解密**,只存密文,即使被拖库也安全
- 主密码忘了 → **恢复码**机制(注册时生成,离线保存)

---

## 3. 数据模型(未来 DB)

```sql
-- vault 用户主记录(首次设主密码时建)
vault_users(user_id PK, email, kdf_params JSON,
            protected_dek TEXT, dek_nonce, verifier,
            pub_sign_key, current_rev BIGINT,
            created_at, updated_at)

-- vault 项(全密文,含 host 配置/密钥/密码)
vault_items(item_id, user_id, kind, ref_id,
            nonce, ct TEXT, signature,
            deleted, revision BIGINT, updated_at)
  PK(user_id, item_id), INDEX(user_id, revision DESC)

-- 订阅状态(主后端库)
subscriptions(user_id PK, plan, quota_limit,
              status, current_period_end,
              source, updated_at)

-- 视图:统计活跃云端主机数(配额用)
CREATE VIEW vault_active_count AS ...  -- WHERE kind='host_meta' AND deleted=0
```

**安全约束**:所有 SQL 带 `WHERE user_id=?`(取自 JWT);日志禁打 `ct/protected_dek/verifier`;`/unlock` 限流防爆破。

---

## 4. API 设计(Go,vault-service)

JWT 复用主后端(HS256 共享 secret 或 RS256 公钥验签)。

```
POST /vault/setup        首次设主密码
POST /vault/unlock       取 verifier + protected_dek(客户端本地比对+解密)
GET  /vault/items?since  增量拉密文
POST /vault/push         推密文项(过配额检查 → applied/conflicts)
GET  /vault/quota        返回 {plan, used, limit, expired_at}   ★ 商业化
POST /vault/recover      恢复码重置主密码(二期)
```

**push 冲突**:服务端比 `base_rev == server_rev`,不等返回 `conflicts`,客户端 per-item merge + 保留双份(密钥数据不敢丢)。

---

## 5. 客户端改造点(结合现有代码)

当前结构:`state.rs` 存 `hosts.json` 明文,`crypto/mod.rs` 用 OS keyring 存密码。未来改造的衔接点:

| 现有 | 未来角色 |
|------|---------|
| `crypto/mod.rs`(keyring) | 降级为「本地免输入缓存」(可选记 DEK 会话),不再是主凭据存储 |
| `state.rs` 的 `hosts.json` | 改为 `vault.local.json`(全密文),`load/save` 走 vault 加解密 |
| `state.rs` 的 `hosts: RwLock<Vec<Host>>` | **不变**(内存仍明文供 UI 用) |
| `models.rs` 的 `Host` | `key_path` 跨设备无效 → vault 里存私钥**内容**,落地成本机文件 |
| `commands/host.rs::connect_host` | **逻辑不变**,只读 `key_path`,文件由 vault 落地 |

新增模块:`vault/{mod,crypto,local}.rs`、`sync/{mod,client,model}.rs`、`commands/vault.rs`、`commands/sync.rs`,注册进 `lib.rs`。

**关键**:阶段 0(加密本地化)做完后,现有功能完全不受影响,只是底层存储换了。

---

## 6. 商业化 / 配额(技术核心)

**铁律:配额只在服务端算,客户端计数仅作提前提示,真正拦截在 `/push`。**

- 配额只计 `kind=host_meta`,密钥/密码跟随主机不算钱
- `subscriptions` 放主后端,vault-service 共享 DB 只读 + LRU 缓存 5 分钟
- 支付成功 → 主后端清 vault-service 缓存(Redis pub/sub 或内部 HTTP)
- Pro 到期:**不删数据**,改报 `ErrSubscriptionExpired`,只读 30 天,降焦虑

### 定价档位建议(国内,Termius 打 3~5 折)

| 档位 | 价格 | 云端配额 |
|------|------|---------|
| Free | 0 | 本地无限 + 云端 **5 台** |
| Pro 月 | ¥9~15 | 无限 + 多设备 + 历史 |
| Pro 年 | ¥88~128 | 同上(主力营收) |
| Pro 终身 | ¥288~388 | 限时促销引流 |
| Teams | ¥39/人/月 | 共享 vault(二期) |

**建议免费版给 5 台而非 0**:0 台用户尝不到甜头不转化;5 台养熟了主机一多自然升级。

---

## 7. 分阶段路线图(重点:标注当前阶段)

```
▼ 现在 ─────────────────────────────────────────────
[ 阶段 A · 基础功能 ] ★ 当前在做
   本地 SSH 连接 / Docker 管理 / 终端 / 文件管理
   凭据走 OS keyring,hosts.json 明文(单机可用)
   完成度:进行中

────────────────────────── 以下为 v2 及以后,暂不启动

[ 阶段 0 · 客户端加密本地化 ]   ~4~6 人日
   不联网。vault 本地存取,跑通加密闭环。
   完成后:本地更安全,功能完全不变。

[ 阶段 1 · Go 后端 + 基础同步 ] ~3~4 人日(Go)+2~3 人日(前端)
   多端能同步密文配置。此阶段即可上「云同步」卖点了。

[ 阶段 2 · 冲突 + 私钥内容同步 ] ~2 人日
   per-item merge + 跨设备私钥落地。真正可用、不丢数据。

[ 阶段 3 · 商业化配额 ]         ~4~7 人日
   subscriptions 表 + 支付回调 + 配额拦截 + quota UI。
   此阶段开始有收入。

[ 阶段 4 · 加固 ]               ~2~3 人日
   恢复码 / 自动锁定 / zeroize / Ed25519 签名 / 指纹核对。

[ 阶段 5 · Docker 部署 + 可观测 ] ~1 人日
   vault-service 容器化 + compose + 健康检查 + 指标。
```

**合计云同步全链路约 10~14 人日,商业化 +4~7 人日。** 每阶段可独立交付、可回退。

---

## 8. 开工前需拍板的决策点(清单,以后逐项定)

> 每项都会单独询问 + 解释为什么,逐个确认后再进入对应阶段。

| # | 决策 | 倾向 | 状态 |
|---|------|------|------|
| 1 | 主密码忘了怎么办 | A 永久丢 / **B 恢复码** / C 服务端可重置 | 待定 |
| 2 | 是否真 E2EE | **A 服务端不可见**(密钥云必须) | 待定 |
| 3 | Host 元数据加密 | **B 加密**(顺带,隐私更好) | 待定 |
| 4 | JWT 共享方式 | A HS256 / **B RS256**(主后端拿私钥) | 待定 |
| 5 | 私钥文件同步 | **A 同步内容**(否则换设备失效) | 待定 |
| 6 | 数据库 | 起步**复用主库**(不同 schema),量大再拆 | 待定 |
| 7 | 免费配额 | **5 台**(平衡获客与转化) | 待定 |
| 8 | 配额计算对象 | **只算 host_meta** | 待定 |
| 9 | 支付渠道 | 国内优先**微信/支付宝** | 待定 |
| 10 | 终身买断 | **做,限时促销**(别常驻) | 待定 |
| 11 | Teams 档 | **二期**(个人版跑通再上) | 待定 |

---

## 9. 明确不做的事(边界)

- ❌ 实时协作编辑(CRDT/Yjs)——配置同步用不到
- ❌ 团队共享 vault——二期
- ❌ CF/EdgeOne 无后端方案——已有服务器,自建最优
- ❌ 盗版/破解防护(起步)——配额在服务端,核心已防
- ❌ 现阶段任何云同步代码——优先把基础功能做扎实

---

## 10. 推进规则(我们之间的约定)

1. **一步步来**:每次只推进一个阶段,不跳跃。
2. **先问后做**:每个需要拍板的关键点(第 8 节 + 各阶段细节),都会单独询问,确认后再动手。
3. **解释 why**:每次提问都附带「为什么这么做 / 不这么做的代价」,而不是只给选项。
4. **当前优先级**:阶段 A(基础功能)优先,云相关全部暂缓,直到基础功能稳定。
5. **文档同步**:每个阶段完成后更新本文档状态,记录实际决策。
