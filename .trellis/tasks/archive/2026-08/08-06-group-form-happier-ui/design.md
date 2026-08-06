# 设计：分组表单页 happier-ui 组件改造

## 组件替换映射

### 1. 外层容器（双栏）
**现状**：
```html
<div class="flex min-h-0 max-h-[32rem] flex-col rounded-lg border border-slate-200">
  <div class="flex items-center justify-between border-b border-slate-100 px-3 py-2">
    <h3 class="text-sm font-medium">可选模型</h3>
    <!-- 右侧操作区/副标题 -->
  </div>
  <div class="min-h-0 flex-1 overflow-y-auto p-3">
    <!-- 内容 -->
  </div>
</div>
```
**目标**：
```html
<HCard variant="outlined" padding="none" class="flex min-h-0 max-h-[32rem] flex-col">
  <template #header>
    <div class="flex items-center justify-between px-3 py-2">
      <h3 class="text-sm font-medium">可选模型</h3>
      <!-- 右侧操作区/副标题 -->
    </div>
  </template>
  <div class="min-h-0 flex-1 overflow-y-auto p-3">
    <!-- 内容 -->
  </div>
</HCard>
```
*注：HCard 内部结构需支持 flex-col 以实现内部滚动。*

### 2. 手风琴条目
**现状**：
```html
<button type="button" class="flex w-full items-center gap-2 px-3 py-2..." @click="toggleProvider(p.id)">
  <ChevronDown class="... transition-transform -rotate-90" />
  <span class="... truncate font-medium text-slate-700">{{ p.name }}</span>
  <span class="...">{{ models.length }} 个模型</span>
</button>
```
**目标**：
```html
<HCell
  clickable
  :title="p.name"
  :show-chevron="false"
  :class="{ 'opacity-50 pointer-events-none': isBound }"
  @click="toggleProvider(p.id)"
>
  <template #prefix>
    <ChevronDown
      :size="14"
      class="text-slate-400 transition-transform"
      :class="{ '-rotate-90': !expandedProviders.has(p.id) }"
    />
  </template>
  <template #suffix>
    <span v-if="ready" class="text-xs text-slate-400">{{ count }} 个模型</span>
  </template>
</HCell>
```
*注：isBound 时可通过 class 或 disabled（如果 HCell 支持）禁用交互。*

### 3. 分数标签
**现状**：
```html
<span class="bg-emerald-50 text-emerald-800 rounded-full px-2 py-0.5 text-[11px]..." title="...">...</span>
```
**目标**：
```html
<HTag
  size="sm"
  :variant="queueDisplayScores[index] ? 'success' : 'default'"
  :title="..."
>
  <template v-if="queueDisplayScores[index]">llm_benchmark · {{ ... }}</template>
  <template v-else>未匹配</template>
</HTag>
```

### 4. 队列项删除按钮
**现状**：
```html
<button class="rounded px-1.5 py-0.5 text-xs text-rose-600 hover:bg-rose-50" @click="removeQueueItem(index)">×</button>
```
**目标**：
```html
<HButton variant="ghost" size="sm" class="text-rose-600 hover:bg-rose-50 hover:text-rose-700" @click="removeQueueItem(index)">×</HButton>
```
*注：HButton 可能有默认 px/py，需覆盖或依赖 size="sm"。*

### 5. 空态
**现状**：
```html
<p class="py-3 text-center text-xs text-slate-400">暂无供应商...</p>
```
**目标**：
```html
<HEmpty class="app-empty-compact" title="暂无供应商，请先到「供应商」页添加" />
```

### 6. 加载态
**现状**：
```html
<div class="py-2 text-xs text-slate-500">正在拉取模型…</div>
```
**目标**：
```html
<HLoading mode="local" size="sm" label="正在拉取模型…" />
```

### 7. 错误块
**现状**：
```html
<div class="rounded-lg border border-rose-200 bg-rose-50 p-4">...</div>
```
**目标**：
```html
<HCard variant="outlined" class="border-rose-200 bg-rose-50">...</HCard>
```
