# BetterTouchscreen 手势使用手册

## 手势生命周期

所有手势遵循统一的三阶段状态模型：

```
Begin → Update → ... → Update → End
```

| 阶段 | 说明 |
|------|------|
| `Begin` | 手指数量/位置满足手势触发条件，手势被识别并开始 |
| `Update` | 手指在屏幕上移动，手势持续更新（delta、scale、rotation 等） |
| `End` | 手指离开屏幕，手势结束 |

## 默认手势

### 单指操作

#### 1.1 光标移动 + 点击（Pointer）

| 属性 | 值 |
|------|-----|
| 手指数 | 1 |
| 触发条件 | 单指在屏幕上触摸即触发 |
| 触地 | 按下左键（BTN_LEFT press），光标开始跟踪手指位移 |
| 移动 | 输出 `REL_X`/`REL_Y` 相对位移事件，光标跟随手指移动（触屏拖拽） |
| 抬起 | 释放左键（BTN_LEFT release），若未移动则形成单击 |
| 输出 | 虚拟触摸板光标事件 |

**使用场景**：模拟触摸板的基本光标操作（移动、点击、拖拽）。程序 grab 触屏后这是唯一的光标输入来源。

### 双指操作

#### 2.1 滚动（Scroll）

| 属性 | 值 |
|------|-----|
| 手指数 | 2 |
| 触发条件 | 两指同时在屏幕上移动 |
| 输出 | 触摸板双指滚动事件（`REL_WHEEL` / `REL_HWHEEL`） |
| 方向 | delta_x → 水平滚动，delta_y → 垂直滚动 |

**使用场景**：网页滚动、文档翻页、列表浏览。

## 手势阈值参数

| 参数 | 默认值 | 说明 |
|------|--------|------|
| `scroll_threshold` | 5.0 px | 双指滚动最小位移 |
| `pinch_threshold` | 0.02 | 捏合最小 scale 变化 |
| `swipe_threshold` | 50.0 px | 滑动最小位移距离 |
| `swipe_deadzone` | 30.0 px | 滑动方向判定的死区范围 |

## 配置文件格式（规划）

```toml
# config.toml

[gestures.scroll]
fingers = 2
threshold = 5.0
natural_scroll = true   # 自然滚动方向

[gestures.pinch]
fingers = 2
threshold = 0.02

[gestures.swipe_3f]
fingers = 3
threshold = 50.0
deadzone = 30.0

[gestures.swipe_4f]
fingers = 4
threshold = 50.0
deadzone = 30.0

[devices]
touchscreen = "/dev/input/eventX"  # 触屏设备路径
```
