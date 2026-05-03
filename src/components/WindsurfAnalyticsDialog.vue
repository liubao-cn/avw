<template>
  <div class="analytics-overlay" @click.self="handleClose">
    <div class="analytics-dialog" @click.stop>
      <div class="dialog-header">
        <div class="header-title">
          <svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor">
            <path d="M19 3H5c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h14c1.1 0 2-.9 2-2V5c0-1.1-.9-2-2-2zM9 17H7v-7h2v7zm4 0h-2V7h2v10zm4 0h-2v-4h2v4z"/>
          </svg>
          <h3>
            {{ $t('windsurf.analyticsTitle') }}
            <span class="account-email" :title="account?.email">{{ account?.email }}</span>
          </h3>
        </div>
        <div class="header-actions">
          <button class="btn small primary" @click="fetchAnalytics" :disabled="isLoading">
            <span v-if="isLoading" class="sync-spinner"></span>
            {{ isLoading ? $t('windsurf.analyticsLoading') : $t('windsurf.analyticsRefresh') }}
          </button>
          <button class="close-btn" @click="handleClose">×</button>
        </div>
      </div>

      <div class="dialog-body">
        <div class="controls">
          <div class="range-presets">
            <button
              v-for="preset in rangePresets"
              :key="preset.days"
              class="range-chip"
              :class="{ active: selectedPresetDays === preset.days }"
              @click="applyPreset(preset.days)"
            >
              {{ preset.label }}
            </button>
          </div>
          <div class="range-inputs">
            <label class="range-field">
              <span class="range-label">{{ $t('windsurf.analyticsStart') }}</span>
              <input type="date" v-model="startDate" class="range-input" />
            </label>
            <label class="range-field">
              <span class="range-label">{{ $t('windsurf.analyticsEnd') }}</span>
              <input type="date" v-model="endDate" class="range-input" />
            </label>
          </div>
        </div>

        <div v-if="errorText" class="alert error">
          {{ errorText }}
        </div>

        <div v-if="!errorText && !hasData && !isLoading" class="alert info">
          {{ $t('windsurf.analyticsEmptyHint') }}
        </div>

        <div v-if="hasData" class="metrics-section">
          <h4 class="section-title">核心指标</h4>
          <div v-if="headlineMetrics.length > 0" class="metric-grid">
            <div v-for="metric in headlineMetrics" :key="metric.key" class="metric-card">
              <div class="metric-label">{{ metric.label }}</div>
              <div class="metric-value">{{ metric.display }}</div>
              <div v-if="metric.sub" class="metric-sub">{{ metric.sub }}</div>
            </div>
          </div>
          <div v-if="hasCompletionStats || hasChatStats" class="detail-stats-grid">
            <div v-if="hasCompletionStats" class="detail-stat-panel">
              <div class="detail-stat-head">代码补全统计</div>
              <div class="detail-stat-list">
                <div class="detail-stat-item">
                  <span>接受次数</span>
                  <strong>{{ formatMetricNumber(analyticsData?.completion_stats?.num_acceptances || 0) }}</strong>
                </div>
                <div class="detail-stat-item">
                  <span>拒绝次数</span>
                  <strong>{{ formatMetricNumber(analyticsData?.completion_stats?.num_rejections || 0) }}</strong>
                </div>
                <div class="detail-stat-item">
                  <span>接受率</span>
                  <strong>{{ formatRate(analyticsData?.completion_stats?.acceptance_rate || 0) }}</strong>
                </div>
                <div class="detail-stat-item">
                  <span>接受行数</span>
                  <strong>{{ formatMetricNumber(analyticsData?.completion_stats?.num_lines_accepted || 0) }}</strong>
                </div>
                <div class="detail-stat-item">
                  <span>活跃天数</span>
                  <strong>{{ formatMetricNumber(analyticsData?.completion_stats?.active_developer_days || 0) }}</strong>
                </div>
              </div>
            </div>

            <div v-if="hasChatStats" class="detail-stat-panel">
              <div class="detail-stat-head">Chat 统计</div>
              <div class="detail-stat-list">
                <div class="detail-stat-item">
                  <span>发送消息</span>
                  <strong>{{ formatMetricNumber(analyticsData?.chat_stats?.chats_sent || 0) }}</strong>
                </div>
                <div class="detail-stat-item">
                  <span>采纳建议</span>
                  <strong>{{ formatMetricNumber(analyticsData?.chat_stats?.chats_accepted || 0) }}</strong>
                </div>
                <div class="detail-stat-item">
                  <span>代码块使用</span>
                  <strong>{{ formatMetricNumber(analyticsData?.chat_stats?.chat_code_blocks_used || 0) }}</strong>
                </div>
                <div class="detail-stat-item">
                  <span>函数解释</span>
                  <strong>{{ formatMetricNumber(analyticsData?.chat_stats?.function_explain_count || 0) }}</strong>
                </div>
                <div class="detail-stat-item">
                  <span>函数重构</span>
                  <strong>{{ formatMetricNumber(analyticsData?.chat_stats?.function_refactor_count || 0) }}</strong>
                </div>
                <div class="detail-stat-item">
                  <span>单元测试</span>
                  <strong>{{ formatMetricNumber(analyticsData?.chat_stats?.function_unit_tests_count || 0) }}</strong>
                </div>
              </div>
            </div>
          </div>

          <div v-else-if="!hasOverviewCharts" class="alert info">
            当前账号返回了分析响应，但还没有可展示的核心趋势数据。
          </div>

          <div v-if="hasOverviewCharts" class="charts-wrapper overview-grid">
            <div v-if="hasDailyLines" class="chart-card chart-span-2">
              <div class="chart-head">
                <span class="chart-title">每日代码行数趋势</span>
                <span class="chart-total">接受 {{ formatMetricNumber(analyticsData?.summary?.total_accepted_lines || 0) }} / 建议 {{ formatMetricNumber(analyticsData?.summary?.total_suggested_lines || 0) }}</span>
              </div>
              <div ref="dailyChartRef" class="chart-canvas chart-canvas-tall"></div>
            </div>

            <div v-if="hasToolUsage" class="chart-card">
              <div class="chart-head">
                <span class="chart-title">工具使用分布</span>
                <span class="chart-total">{{ analyticsData?.tool_usage?.length || 0 }} 个工具</span>
              </div>
              <div ref="toolChartRef" class="chart-canvas"></div>
            </div>

            <div v-if="hasModelUsage" class="chart-card">
              <div class="chart-head">
                <span class="chart-title">模型使用分布</span>
                <span class="chart-total">{{ analyticsData?.model_usage_summary?.length || 0 }} 个模型</span>
              </div>
              <div ref="modelChartRef" class="chart-canvas"></div>
            </div>

            <div v-if="hasCompletionsByDay" class="chart-card">
              <div class="chart-head">
                <span class="chart-title">每日补全趋势</span>
                <span class="chart-total">{{ analyticsData?.completions_by_day?.length || 0 }} 天</span>
              </div>
              <div ref="completionsByDayChartRef" class="chart-canvas"></div>
            </div>

            <div v-if="hasChatsByDay" class="chart-card">
              <div class="chart-head">
                <span class="chart-title">每日 Chat 趋势</span>
                <span class="chart-total">{{ analyticsData?.chats_by_day?.length || 0 }} 天</span>
              </div>
              <div ref="chatsByDayChartRef" class="chart-canvas"></div>
            </div>

            <div v-if="hasChatsByModel" class="chart-card chart-span-2">
              <div class="chart-head">
                <span class="chart-title">按模型 Chat 统计</span>
                <span class="chart-total">{{ analyticsData?.chats_by_model?.length || 0 }} 个模型</span>
              </div>
              <div ref="chatModelChartRef" class="chart-canvas"></div>
            </div>

            <div v-if="hasCompletionsByLanguage" class="chart-card chart-span-2">
              <div class="chart-head">
                <span class="chart-title">按语言补全统计</span>
                <span class="chart-total">{{ analyticsData?.completions_by_language?.length || 0 }} 种语言</span>
              </div>
              <div ref="languageChartRef" class="chart-canvas"></div>
            </div>
          </div>

          <div class="meta-row">
            <span v-if="provider" class="meta-chip" :class="`provider-${provider}`">
              {{ $t('windsurf.analyticsProvider', { provider }) }}
            </span>
            <span class="meta-chip window">
              {{ formatUnix(result?.start_timestamp) }} ~ {{ formatUnix(result?.end_timestamp) }}
            </span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, computed, nextTick, onBeforeUnmount, onMounted, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useI18n } from 'vue-i18n'
import * as echarts from 'echarts'
import { useEscToClose } from '../composables/useEscToClose'

const { t } = useI18n()

const props = defineProps({
  account: {
    type: Object,
    required: true
  }
})
const emit = defineEmits(['close', 'show-toast'])

const rangePresets = computed(() => [
  { days: 7, label: t('windsurf.analyticsPresetDays', { days: 7 }) },
  { days: 14, label: t('windsurf.analyticsPresetDays', { days: 14 }) },
  { days: 30, label: t('windsurf.analyticsPresetDays', { days: 30 }) },
  { days: 90, label: t('windsurf.analyticsPresetDays', { days: 90 }) }
])

const isLoading = ref(false)
const errorText = ref('')
const result = ref(null)
const selectedPresetDays = ref(30)

const startDate = ref('')
const endDate = ref('')

const toDateStr = (d) => {
  const pad = (n) => String(n).padStart(2, '0')
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`
}

const initDefaultRange = () => {
  const now = new Date()
  const start = new Date(now)
  start.setDate(start.getDate() - 30)
  startDate.value = toDateStr(start)
  endDate.value = toDateStr(now)
}

const applyPreset = (days) => {
  selectedPresetDays.value = days
  const now = new Date()
  const start = new Date(now)
  start.setDate(start.getDate() - days)
  startDate.value = toDateStr(start)
  endDate.value = toDateStr(now)
  // 立即刷新
  fetchAnalytics()
}

const formatUnix = (sec) => {
  if (!sec) return '-'
  const d = new Date(sec * 1000)
  if (Number.isNaN(d.getTime())) return '-'
  return d.toLocaleString()
}

const provider = computed(() => result.value?.provider || '')

const analyticsData = computed(() => result.value?.analytics_data || null)
const hasData = computed(() => Boolean(result.value && result.value.success && analyticsData.value))

const formatMetricNumber = (value, digits = 0) => {
  const num = Number(value)
  if (!Number.isFinite(num)) return '-'
  return digits > 0 ? num.toFixed(digits) : Math.round(num).toLocaleString()
}

const formatRate = (value) => {
  const num = Number(value)
  if (!Number.isFinite(num)) return '-'
  const percent = num > 0 && num <= 1 ? num * 100 : num
  return `${percent.toFixed(1)}%`
}

const formatToolName = (value) => {
  if (!value) return '-'
  return String(value)
    .replace(/_/g, ' ')
    .toLowerCase()
    .replace(/\b\w/g, (char) => char.toUpperCase())
}

const formatModelName = (value) => value || '-'

const headlineMetrics = computed(() => {
  const summary = analyticsData.value?.summary || {}
  return [
    {
      key: 'accepted-lines',
      label: '总接受代码行数',
      display: formatMetricNumber(summary.total_accepted_lines || 0),
      sub: summary.total_suggested_lines > 0 ? `建议 ${formatMetricNumber(summary.total_suggested_lines)}` : ''
    },
    {
      key: 'avg-lines',
      label: '日均代码行数',
      display: formatMetricNumber(summary.avg_daily_accepted_lines || 0, 1),
      sub: analyticsData.value?.daily_cascade_lines?.length ? `${analyticsData.value.daily_cascade_lines.length} 天数据` : ''
    },
    {
      key: 'tokens',
      label: '总 Token 消耗',
      display: summary.total_tokens > 0 ? formatMetricNumber((summary.total_tokens || 0) / 100) : '-',
      sub: summary.total_tokens > 0 ? '按模型明细聚合' : ''
    },
    {
      key: 'primary-model',
      label: '主要模型',
      display: formatModelName(summary.primary_model),
      sub: analyticsData.value?.model_usage_summary?.length ? `${analyticsData.value.model_usage_summary.length} 个模型` : ''
    },
    {
      key: 'primary-tool',
      label: '主要工具',
      display: formatToolName(summary.primary_tool),
      sub: analyticsData.value?.tool_usage?.length ? `${analyticsData.value.tool_usage.length} 个工具` : ''
    }
  ]
})

const hasCompletionStats = computed(() => {
  const stats = analyticsData.value?.completion_stats
  return Boolean(stats && (
    Number(stats.num_acceptances || 0) > 0 ||
    Number(stats.num_rejections || 0) > 0 ||
    Number(stats.num_lines_accepted || 0) > 0
  ))
})

const hasChatStats = computed(() => {
  const stats = analyticsData.value?.chat_stats
  return Boolean(stats && Number(stats.chats_sent || 0) > 0)
})

const hasDailyLines = computed(() => (analyticsData.value?.daily_cascade_lines?.length || 0) > 0)
const hasToolUsage = computed(() => (analyticsData.value?.tool_usage?.length || 0) > 0)
const hasModelUsage = computed(() => (analyticsData.value?.model_usage_summary?.length || 0) > 0)
const hasCompletionsByDay = computed(() => (analyticsData.value?.completions_by_day?.length || 0) > 0)
const hasChatsByDay = computed(() => (analyticsData.value?.chats_by_day?.length || 0) > 0)
const hasChatsByModel = computed(() => (analyticsData.value?.chats_by_model?.length || 0) > 0)
const hasCompletionsByLanguage = computed(() => (analyticsData.value?.completions_by_language?.length || 0) > 0)
const hasOverviewCharts = computed(() => (
  hasDailyLines.value ||
  hasToolUsage.value ||
  hasModelUsage.value ||
  hasCompletionsByDay.value ||
  hasChatsByDay.value ||
  hasChatsByModel.value ||
  hasCompletionsByLanguage.value
))

const dailyChartRef = ref(null)
const toolChartRef = ref(null)
const modelChartRef = ref(null)
const completionsByDayChartRef = ref(null)
const chatsByDayChartRef = ref(null)
const chatModelChartRef = ref(null)
const languageChartRef = ref(null)

let dailyChart = null
let toolChart = null
let modelChart = null
let completionsByDayChart = null
let chatsByDayChart = null
let chatModelChart = null
let languageChart = null

const disposeCharts = () => {
  for (const chart of [dailyChart, toolChart, modelChart, completionsByDayChart, chatsByDayChart, chatModelChart, languageChart]) {
    chart?.dispose()
  }
  dailyChart = null
  toolChart = null
  modelChart = null
  completionsByDayChart = null
  chatsByDayChart = null
  chatModelChart = null
  languageChart = null
}

const resizeCharts = () => {
  for (const chart of [dailyChart, toolChart, modelChart, completionsByDayChart, chatsByDayChart, chatModelChart, languageChart]) {
    chart?.resize()
  }
}

const initDailyChart = () => {
  if (!dailyChartRef.value || !hasDailyLines.value) return
  dailyChart?.dispose()
  dailyChart = echarts.init(dailyChartRef.value)

  const dates = analyticsData.value.daily_cascade_lines.map((item) => item.date)
  const acceptedLines = analyticsData.value.daily_cascade_lines.map((item) => Number(item.accepted_lines || 0))
  const suggestedLines = analyticsData.value.daily_cascade_lines.map((item) => Number(item.suggested_lines || 0))

  dailyChart.setOption({
    tooltip: {
      trigger: 'axis',
      backgroundColor: 'rgba(255,255,255,0.96)',
      borderColor: '#e5e7eb',
      borderWidth: 1,
      textStyle: { color: '#111827' },
      formatter: (params) => {
        const lines = Array.isArray(params) ? params : [params]
        const date = lines[0]?.axisValueLabel || '-'
        return [
          `<div style="font-weight:600;margin-bottom:6px">${date}</div>`,
          ...lines.map((item) => `${item.marker}${item.seriesName}：${formatMetricNumber(item.data)}`)
        ].join('<br/>')
      }
    },
    legend: {
      top: 0,
      right: 0,
      data: ['接受代码行数', '建议代码行数']
    },
    grid: {
      left: '3%',
      right: '4%',
      bottom: '3%',
      top: '16%',
      containLabel: true
    },
    xAxis: {
      type: 'category',
      data: dates,
      boundaryGap: false,
      axisLabel: { color: '#6b7280' }
    },
    yAxis: {
      type: 'value',
      axisLabel: { color: '#6b7280' },
      splitLine: { lineStyle: { color: '#e5e7eb', type: 'dashed' } }
    },
    series: [
      {
        name: '接受代码行数',
        type: 'line',
        smooth: true,
        showSymbol: false,
        data: acceptedLines,
        lineStyle: { width: 3, color: '#5b86e5' },
        itemStyle: { color: '#5b86e5' },
        areaStyle: {
          color: {
            type: 'linear',
            x: 0,
            y: 0,
            x2: 0,
            y2: 1,
            colorStops: [
              { offset: 0, color: 'rgba(91, 134, 229, 0.35)' },
              { offset: 1, color: 'rgba(91, 134, 229, 0.04)' }
            ]
          }
        }
      },
      {
        name: '建议代码行数',
        type: 'line',
        smooth: true,
        showSymbol: false,
        data: suggestedLines,
        lineStyle: { width: 3, color: '#36d1dc', type: 'dashed' },
        itemStyle: { color: '#36d1dc' }
      }
    ]
  })
}

const initToolChart = () => {
  if (!toolChartRef.value || !hasToolUsage.value) return
  toolChart?.dispose()
  toolChart = echarts.init(toolChartRef.value)

  const data = analyticsData.value.tool_usage.map((item) => ({
    name: formatToolName(item.tool_name),
    value: Number(item.count || 0)
  }))

  toolChart.setOption({
    tooltip: {
      trigger: 'item',
      formatter: ({ name, value, percent }) => `${name}<br/>调用次数：${formatMetricNumber(value)}<br/>占比：${Number(percent || 0).toFixed(1)}%`
    },
    legend: {
      type: 'scroll',
      orient: 'vertical',
      right: 0,
      top: 'middle'
    },
    series: [{
      name: '工具使用',
      type: 'pie',
      radius: ['42%', '70%'],
      center: ['38%', '50%'],
      avoidLabelOverlap: false,
      label: { show: false },
      emphasis: { label: { show: true, fontSize: 14, fontWeight: 'bold' } },
      data
    }]
  })
}

const initModelChart = () => {
  if (!modelChartRef.value || !hasModelUsage.value) return
  modelChart?.dispose()
  modelChart = echarts.init(modelChartRef.value)

  const models = analyticsData.value.model_usage_summary.map((item) => formatModelName(item.model_name))
  const counts = analyticsData.value.model_usage_summary.map((item) => Number(item.total_count || 0))
  const percentages = analyticsData.value.model_usage_summary.map((item) => Number(item.percentage || 0))

  modelChart.setOption({
    tooltip: {
      trigger: 'axis',
      axisPointer: { type: 'shadow' },
      formatter: (params) => {
        const row = Array.isArray(params) ? params[0] : params
        const index = row?.dataIndex ?? 0
        return [
          `<div style="font-weight:600;margin-bottom:6px">${models[index] || '-'}</div>`,
          `使用次数：${formatMetricNumber(counts[index] || 0)}`,
          `占比：${(percentages[index] || 0).toFixed(1)}%`
        ].join('<br/>')
      }
    },
    grid: {
      left: '3%',
      right: '12%',
      bottom: '3%',
      top: '4%',
      containLabel: true
    },
    xAxis: {
      type: 'value',
      axisLabel: { color: '#6b7280' },
      splitLine: { lineStyle: { color: '#e5e7eb', type: 'dashed' } }
    },
    yAxis: {
      type: 'category',
      data: models,
      axisLabel: { color: '#374151', width: 140, overflow: 'truncate' }
    },
    series: [{
      type: 'bar',
      data: counts,
      barWidth: 18,
      itemStyle: {
        color: new echarts.graphic.LinearGradient(0, 0, 1, 0, [
          { offset: 0, color: '#36d1dc' },
          { offset: 1, color: '#5b86e5' }
        ]),
        borderRadius: [0, 4, 4, 0]
      },
      label: {
        show: true,
        position: 'right',
        formatter: (params) => `${(percentages[params.dataIndex] || 0).toFixed(1)}%`,
        color: '#6b7280'
      }
    }]
  })
}

const initCompletionsByDayChart = () => {
  if (!completionsByDayChartRef.value || !hasCompletionsByDay.value) return
  completionsByDayChart?.dispose()
  completionsByDayChart = echarts.init(completionsByDayChartRef.value)

  const dates = analyticsData.value.completions_by_day.map((item) => item.date)
  const acceptances = analyticsData.value.completions_by_day.map((item) => Number(item.statistics?.num_acceptances || 0))
  const rejections = analyticsData.value.completions_by_day.map((item) => Number(item.statistics?.num_rejections || 0))

  completionsByDayChart.setOption({
    tooltip: { trigger: 'axis', axisPointer: { type: 'shadow' } },
    legend: { top: 0, data: ['接受', '拒绝'] },
    grid: { left: '3%', right: '4%', bottom: '3%', top: '16%', containLabel: true },
    xAxis: { type: 'category', data: dates, axisLabel: { color: '#6b7280' } },
    yAxis: { type: 'value', axisLabel: { color: '#6b7280' } },
    series: [
      { name: '接受', type: 'bar', stack: 'total', data: acceptances, itemStyle: { color: '#67c23a' } },
      { name: '拒绝', type: 'bar', stack: 'total', data: rejections, itemStyle: { color: '#f56c6c' } }
    ]
  })
}

const initChatsByDayChart = () => {
  if (!chatsByDayChartRef.value || !hasChatsByDay.value) return
  chatsByDayChart?.dispose()
  chatsByDayChart = echarts.init(chatsByDayChartRef.value)

  const dates = analyticsData.value.chats_by_day.map((item) => item.date)
  const chatsSent = analyticsData.value.chats_by_day.map((item) => Number(item.stats?.chats_sent || 0))

  chatsByDayChart.setOption({
    tooltip: { trigger: 'axis', axisPointer: { type: 'line' } },
    grid: { left: '3%', right: '4%', bottom: '3%', top: '12%', containLabel: true },
    xAxis: { type: 'category', data: dates, axisLabel: { color: '#6b7280' } },
    yAxis: { type: 'value', axisLabel: { color: '#6b7280' } },
    series: [{
      name: 'Chat 消息数',
      type: 'line',
      smooth: true,
      showSymbol: false,
      data: chatsSent,
      lineStyle: { width: 3, color: '#8b5cf6' },
      itemStyle: { color: '#8b5cf6' }
    }]
  })
}

const initChatModelChart = () => {
  if (!chatModelChartRef.value || !hasChatsByModel.value) return
  chatModelChart?.dispose()
  chatModelChart = echarts.init(chatModelChartRef.value)

  const data = analyticsData.value.chats_by_model.map((item) => ({
    name: formatModelName(item.model_name),
    value: Number(item.stats?.chats_sent || 0)
  }))

  chatModelChart.setOption({
    tooltip: {
      trigger: 'item',
      formatter: ({ name, value, percent }) => `${name}<br/>Chat 次数：${formatMetricNumber(value)}<br/>占比：${Number(percent || 0).toFixed(1)}%`
    },
    legend: {
      type: 'scroll',
      orient: 'vertical',
      right: 0,
      top: 20,
      bottom: 20
    },
    series: [{
      type: 'pie',
      radius: ['40%', '72%'],
      center: ['38%', '50%'],
      avoidLabelOverlap: false,
      label: { show: false },
      emphasis: { label: { show: true, fontSize: 14, fontWeight: 'bold' } },
      data
    }]
  })
}

const initLanguageChart = () => {
  if (!languageChartRef.value || !hasCompletionsByLanguage.value) return
  languageChart?.dispose()
  languageChart = echarts.init(languageChartRef.value)

  const data = analyticsData.value.completions_by_language || []
  const languages = data.map((item) => item.language_name || '-')
  const acceptances = data.map((item) => Number(item.statistics?.num_acceptances || 0))
  const rejections = data.map((item) => Number(item.statistics?.num_rejections || 0))

  languageChart.setOption({
    tooltip: {
      trigger: 'axis',
      axisPointer: { type: 'shadow' },
      formatter: (params) => {
        const rows = Array.isArray(params) ? params : [params]
        const first = rows[0]
        const index = first?.dataIndex ?? 0
        return [
          `<div style="font-weight:600;margin-bottom:6px">${languages[index] || '-'}</div>`,
          `接受：${formatMetricNumber(acceptances[index] || 0)}`,
          `拒绝：${formatMetricNumber(rejections[index] || 0)}`
        ].join('<br/>')
      }
    },
    legend: { top: 0, data: ['接受', '拒绝'] },
    grid: { left: '3%', right: '4%', bottom: '3%', top: '16%', containLabel: true },
    xAxis: {
      type: 'category',
      data: languages,
      axisLabel: { color: '#374151', interval: 0, rotate: languages.length > 6 ? 20 : 0 }
    },
    yAxis: {
      type: 'value',
      axisLabel: { color: '#6b7280' },
      splitLine: { lineStyle: { color: '#e5e7eb', type: 'dashed' } }
    },
    series: [
      {
        name: '接受',
        type: 'bar',
        data: acceptances,
        barMaxWidth: 26,
        itemStyle: { color: '#67c23a', borderRadius: [4, 4, 0, 0] }
      },
      {
        name: '拒绝',
        type: 'bar',
        data: rejections,
        barMaxWidth: 26,
        itemStyle: { color: '#f56c6c', borderRadius: [4, 4, 0, 0] }
      }
    ]
  })
}

const initCharts = async () => {
  if (!hasData.value) {
    disposeCharts()
    return
  }
  await nextTick()
  initDailyChart()
  initToolChart()
  initModelChart()
  initCompletionsByDayChart()
  initChatsByDayChart()
  initChatModelChart()
  initLanguageChart()
  resizeCharts()
}

// 把 yyyy-mm-dd 转 Unix 秒（UTC 当日 00:00 或末日 23:59:59）
const toUnixSeconds = (dateStr, endOfDay = false) => {
  if (!dateStr) return 0
  const d = new Date(`${dateStr}T${endOfDay ? '23:59:59' : '00:00:00'}`)
  if (Number.isNaN(d.getTime())) return 0
  return Math.floor(d.getTime() / 1000)
}

const fetchAnalytics = async () => {
  if (isLoading.value) return
  const acc = props.account
  if (!acc || !acc.auth_token) {
    errorText.value = t('windsurf.analyticsNoAuth')
    emit('show-toast', errorText.value, 'error')
    return
  }

  const startTs = toUnixSeconds(startDate.value, false)
  const endTs = toUnixSeconds(endDate.value, true)
  if (!startTs || !endTs || startTs >= endTs) {
    errorText.value = t('windsurf.analyticsRangeInvalid')
    return
  }

  isLoading.value = true
  errorText.value = ''
  try {
    const resp = await invoke('windsurf_get_analytics', {
      authToken: acc.auth_token,
      auth_token: acc.auth_token,
      devinAccountId: acc.devin_account_id || null,
      devin_account_id: acc.devin_account_id || null,
      devinAuth1Token: acc.auth_token?.startsWith('auth1_') ? acc.auth_token : null,
      devin_auth1_token: acc.auth_token?.startsWith('auth1_') ? acc.auth_token : null,
      devinPrimaryOrgId: acc.devin_primary_org_id || null,
      devin_primary_org_id: acc.devin_primary_org_id || null,
      sessionToken: acc.session_token || null,
      session_token: acc.session_token || null,
      startTimestamp: startTs,
      start_timestamp: startTs,
      endTimestamp: endTs,
      end_timestamp: endTs
    })

    if (!resp) {
      errorText.value = t('windsurf.analyticsUnknownError')
      return
    }
    result.value = resp
    if (!resp.success) {
      errorText.value = resp.error || t('windsurf.analyticsUnknownError')
      disposeCharts()
    } else {
      await initCharts()
    }
  } catch (e) {
    errorText.value = `${e.message || e}`
    disposeCharts()
  } finally {
    isLoading.value = false
  }
}

const handleClose = () => {
  emit('close')
}

useEscToClose(() => true, handleClose)

watch(
  () => props.account?.email,
  () => {
    disposeCharts()
    result.value = null
    errorText.value = ''
    initDefaultRange()
    fetchAnalytics()
  }
)

watch(analyticsData, () => {
  initCharts()
})

const handleWindowResize = () => resizeCharts()

onMounted(() => {
  window.addEventListener('resize', handleWindowResize)
  initDefaultRange()
  fetchAnalytics()
})

onBeforeUnmount(() => {
  window.removeEventListener('resize', handleWindowResize)
  disposeCharts()
})
</script>

<style scoped>
.analytics-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.5);
  z-index: 3500;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 20px;
}

.analytics-dialog {
  background: var(--card-bg, #fff);
  border-radius: 14px;
  width: min(960px, 96vw);
  max-height: 92vh;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  box-shadow: 0 20px 40px rgba(0, 0, 0, 0.18);
}

.dialog-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 14px 16px;
  border-bottom: 1px solid rgba(0, 0, 0, 0.06);
  gap: 12px;
}

.header-title {
  display: flex;
  align-items: center;
  gap: 10px;
  min-width: 0;
}

.header-title h3 {
  margin: 0;
  font-size: 16px;
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
}

.account-email {
  font-size: 12px;
  font-weight: 500;
  color: rgba(0, 0, 0, 0.55);
  padding: 2px 8px;
  border-radius: 999px;
  background: rgba(0, 0, 0, 0.05);
  max-width: 260px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.header-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}

.close-btn {
  border: none;
  background: transparent;
  font-size: 22px;
  cursor: pointer;
  opacity: 0.7;
  width: 32px;
  height: 32px;
  border-radius: 10px;
}

.close-btn:hover {
  background: rgba(0, 0, 0, 0.05);
  opacity: 1;
}

.dialog-body {
  padding: 16px;
  overflow: auto;
  display: flex;
  flex-direction: column;
  gap: 14px;
  flex: 1;
}

.controls {
  display: flex;
  flex-direction: column;
  gap: 10px;
  padding: 12px;
  border: 1px solid rgba(0, 0, 0, 0.06);
  border-radius: 12px;
  background: rgba(0, 0, 0, 0.02);
}

.range-presets {
  display: flex;
  gap: 6px;
  flex-wrap: wrap;
}

.range-chip {
  border: none;
  background: rgba(0, 0, 0, 0.06);
  color: inherit;
  padding: 6px 12px;
  border-radius: 999px;
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  transition: background 0.15s;
}

.range-chip:hover { background: rgba(0, 0, 0, 0.1); }
.range-chip.active {
  background: rgba(59, 130, 246, 0.18);
  color: #2563eb;
}

.range-inputs {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
  align-items: center;
}

.range-field {
  display: flex;
  flex-direction: column;
  gap: 4px;
  font-size: 12px;
  color: rgba(0, 0, 0, 0.7);
}

.range-label {
  font-size: 12px;
  opacity: 0.75;
}

.range-input {
  padding: 6px 8px;
  border-radius: 8px;
  border: 1px solid rgba(0, 0, 0, 0.1);
  background: #fff;
  font-size: 13px;
}

.alert {
  padding: 10px 12px;
  border-radius: 10px;
  font-size: 13px;
}

.alert.error {
  background: rgba(239, 68, 68, 0.12);
  color: #991b1b;
}

.alert.info {
  background: rgba(59, 130, 246, 0.1);
  color: #1d4ed8;
}

.metrics-section {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.section-title {
  margin: 0;
  font-size: 14px;
  color: rgba(0, 0, 0, 0.65);
}

.metric-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
  gap: 10px;
}

.metric-card {
  padding: 12px;
  border-radius: 12px;
  background: rgba(59, 130, 246, 0.06);
  border: 1px solid rgba(59, 130, 246, 0.12);
}

.metric-label {
  font-size: 12px;
  color: rgba(0, 0, 0, 0.55);
  margin-bottom: 6px;
  font-weight: 600;
}

.metric-value {
  font-size: 22px;
  font-weight: 700;
  color: #1e40af;
}

.metric-sub {
  font-size: 11px;
  color: rgba(0, 0, 0, 0.45);
  margin-top: 4px;
}

.detail-stats-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
  gap: 10px;
}

.detail-stat-panel {
  border: 1px solid rgba(0, 0, 0, 0.08);
  border-radius: 12px;
  background: rgba(255, 255, 255, 0.96);
  padding: 12px;
}

.detail-stat-head {
  font-size: 13px;
  font-weight: 700;
  color: rgba(0, 0, 0, 0.72);
  margin-bottom: 10px;
}

.detail-stat-list {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 8px;
}

.detail-stat-item {
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 10px;
  border-radius: 10px;
  background: rgba(59, 130, 246, 0.05);
}

.detail-stat-item span {
  font-size: 12px;
  color: rgba(0, 0, 0, 0.55);
}

.detail-stat-item strong {
  font-size: 18px;
  color: #1f2937;
}

.charts-wrapper {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.overview-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 12px;
}

.chart-span-2 {
  grid-column: span 2;
}

.chart-card {
  border: 1px solid rgba(0, 0, 0, 0.08);
  border-radius: 12px;
  padding: 10px 12px;
  background: #fff;
}

.chart-head {
  display: flex;
  justify-content: space-between;
  align-items: baseline;
  margin-bottom: 6px;
}

.chart-title {
  font-size: 12px;
  color: rgba(0, 0, 0, 0.7);
  font-weight: 600;
}

.chart-total {
  font-size: 11px;
  color: rgba(0, 0, 0, 0.5);
}

.chart-canvas {
  width: 100%;
  height: 260px;
  display: block;
}

.chart-canvas-tall {
  height: 320px;
}

.meta-row {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
}

.meta-chip {
  padding: 4px 10px;
  border-radius: 999px;
  font-size: 11px;
  font-weight: 600;
  background: rgba(0, 0, 0, 0.06);
  color: rgba(0, 0, 0, 0.65);
}

.meta-chip.provider-auth1 {
  background: rgba(16, 185, 129, 0.14);
  color: #047857;
}

.btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 8px 16px;
  border-radius: 6px;
  border: none;
  cursor: pointer;
  font-size: 14px;
  font-weight: 500;
  transition: all 0.2s;
  margin: 0;
}

.btn.small {
  padding: 6px 12px;
  font-size: 13px;
}

.btn.primary {
  background: var(--primary-color, #3b82f6);
  color: white;
}

.btn.primary:hover {
  background: var(--primary-hover, #2563eb);
}

.btn:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.sync-spinner {
  width: 14px;
  height: 14px;
  border: 2px solid rgba(255, 255, 255, 0.3);
  border-top-color: white;
  border-radius: 50%;
  animation: spin 1s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

@media (max-width: 900px) {
  .overview-grid {
    grid-template-columns: 1fr;
  }

  .chart-span-2 {
    grid-column: span 1;
  }
}

@media (max-width: 640px) {
  .detail-stat-list {
    grid-template-columns: 1fr;
  }

  .chart-head {
    flex-direction: column;
    gap: 4px;
    align-items: flex-start;
  }

  .metric-grid {
    grid-template-columns: 1fr;
  }
}

/* ============================== */
/*  暗色主题统一覆盖               */
/* ============================== */

[data-theme='dark'] .analytics-overlay {
  background: rgba(0, 0, 0, 0.68);
}

[data-theme='dark'] .analytics-dialog {
  background: var(--color-surface, #1e293b);
  color: var(--color-text-primary, #e2e8f0);
  border: 1px solid rgba(148, 163, 184, 0.2);
  box-shadow: 0 20px 40px rgba(0, 0, 0, 0.55);
}

[data-theme='dark'] .dialog-header {
  border-bottom-color: rgba(148, 163, 184, 0.2);
}

[data-theme='dark'] .header-title h3 {
  color: var(--color-text-heading, #e2e8f0);
}

[data-theme='dark'] .account-email {
  color: var(--color-text-muted, #94a3b8);
}

[data-theme='dark'] .close-btn {
  color: var(--color-text-secondary, #cbd5f5);
}

[data-theme='dark'] .close-btn:hover {
  background: rgba(255, 255, 255, 0.08);
}

[data-theme='dark'] .controls {
  background: rgba(148, 163, 184, 0.08);
}

[data-theme='dark'] .range-chip {
  background: rgba(148, 163, 184, 0.16);
  color: var(--color-text-primary, #e2e8f0);
}

[data-theme='dark'] .range-chip:hover {
  background: rgba(148, 163, 184, 0.24);
}

[data-theme='dark'] .range-chip.active {
  background: rgba(96, 165, 250, 0.26);
  color: #93c5fd;
}

[data-theme='dark'] .range-field {
  color: var(--color-text-primary, #e2e8f0);
}

[data-theme='dark'] .range-label {
  color: var(--color-text-secondary, #cbd5f5);
}

[data-theme='dark'] .range-input {
  background: var(--color-input-bg, rgba(15, 23, 42, 0.85));
  border-color: var(--color-input-border, rgba(148, 163, 184, 0.35));
  color: var(--color-text-primary, #e2e8f0);
}

[data-theme='dark'] .range-input:focus {
  border-color: var(--color-input-border-focus, #6366f1);
}

[data-theme='dark'] .alert.error {
  background: rgba(248, 113, 113, 0.18);
  color: #fca5a5;
}

[data-theme='dark'] .alert.info {
  background: rgba(96, 165, 250, 0.16);
  color: #93c5fd;
}

[data-theme='dark'] .section-title {
  color: var(--color-text-secondary, #cbd5f5);
}

[data-theme='dark'] .metric-card {
  background: rgba(96, 165, 250, 0.1);
  border-color: rgba(96, 165, 250, 0.22);
}

[data-theme='dark'] .metric-label {
  color: var(--color-text-secondary, #cbd5f5);
}

[data-theme='dark'] .metric-value {
  color: #93c5fd;
}

[data-theme='dark'] .metric-sub {
  color: var(--color-text-muted, #94a3b8);
}

[data-theme='dark'] .detail-stat-panel {
  background: rgba(148, 163, 184, 0.08);
  border-color: rgba(148, 163, 184, 0.18);
}

[data-theme='dark'] .detail-stat-head {
  color: var(--color-text-primary, #e2e8f0);
}

[data-theme='dark'] .detail-stat-item {
  background: rgba(96, 165, 250, 0.1);
}

[data-theme='dark'] .detail-stat-item span {
  color: var(--color-text-muted, #94a3b8);
}

[data-theme='dark'] .detail-stat-item strong {
  color: var(--color-text-strong, #f8fafc);
}

[data-theme='dark'] .chart-card {
  background: rgba(148, 163, 184, 0.06);
  border-color: rgba(148, 163, 184, 0.18);
}

[data-theme='dark'] .chart-title {
  color: var(--color-text-secondary, #cbd5f5);
}

[data-theme='dark'] .chart-total {
  color: var(--color-text-muted, #94a3b8);
}

[data-theme='dark'] .meta-chip {
  background: rgba(148, 163, 184, 0.14);
  color: var(--color-text-primary, #e2e8f0);
}

[data-theme='dark'] .meta-chip.provider-auth1 {
  background: rgba(45, 212, 191, 0.18);
  color: #5eead4;
}
</style>
