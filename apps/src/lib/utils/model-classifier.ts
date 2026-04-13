/**
 * 模型分类工具
 * 根据模型名称自动识别模型类型
 */

export type ModelCategory =
  | "chatgpt"
  | "claude"
  | "glm"
  | "gemini"
  | "deepseek"
  | "qwen"
  | "grok"
  | "kimi"
  | "other";

export interface ModelCategoryInfo {
  key: ModelCategory;
  label: string;
  color: string;
  bgColor: string;
}

// 模型分类规则
const MODEL_PATTERNS: Record<ModelCategory, RegExp[]> = {
  chatgpt: [
    /^gpt-/i,
    /^o1-/i,
    /^chatgpt/i,
    /openai/i,
  ],
  claude: [
    /^claude/i,
    /anthropic/i,
  ],
  glm: [
    /^glm/i,
    /^chatglm/i,
    /zhipu/i,
  ],
  gemini: [
    /^gemini/i,
    /^palm/i,
    /google/i,
  ],
  deepseek: [
    /^deepseek/i,
  ],
  qwen: [
    /^qwen/i,
    /^tongyi/i,
    /alibaba/i,
  ],
  grok: [
    /^grok/i,
    /^xai/i,
    /twitter/i,
  ],
  kimi: [
    /^kimi/i,
    /^moonshot/i,
    /月之暗面/i,
  ],
  other: [],
};

// 分类信息配置
export const MODEL_CATEGORY_INFO: Record<ModelCategory, ModelCategoryInfo> = {
  chatgpt: {
    key: "chatgpt",
    label: "ChatGPT",
    color: "text-green-600",
    bgColor: "bg-green-500/10",
  },
  claude: {
    key: "claude",
    label: "Claude",
    color: "text-purple-600",
    bgColor: "bg-purple-500/10",
  },
  glm: {
    key: "glm",
    label: "GLM",
    color: "text-blue-600",
    bgColor: "bg-blue-500/10",
  },
  gemini: {
    key: "gemini",
    label: "Gemini",
    color: "text-orange-600",
    bgColor: "bg-orange-500/10",
  },
  deepseek: {
    key: "deepseek",
    label: "DeepSeek",
    color: "text-cyan-600",
    bgColor: "bg-cyan-500/10",
  },
  qwen: {
    key: "qwen",
    label: "Qwen",
    color: "text-pink-600",
    bgColor: "bg-pink-500/10",
  },
  grok: {
    key: "grok",
    label: "Grok",
    color: "text-indigo-600",
    bgColor: "bg-indigo-500/10",
  },
  kimi: {
    key: "kimi",
    label: "Kimi",
    color: "text-teal-600",
    bgColor: "bg-teal-500/10",
  },
  other: {
    key: "other",
    label: "Other",
    color: "text-gray-600",
    bgColor: "bg-gray-500/10",
  },
};

/**
 * 根据模型名称识别分类
 */
export function classifyModel(modelName: string): ModelCategory {
  const normalizedName = modelName.trim().toLowerCase();

  for (const [category, patterns] of Object.entries(MODEL_PATTERNS)) {
    if (category === "other") continue;

    for (const pattern of patterns) {
      if (pattern.test(normalizedName)) {
        return category as ModelCategory;
      }
    }
  }

  return "other";
}

/**
 * 获取分类信息
 */
export function getCategoryInfo(category: ModelCategory): ModelCategoryInfo {
  return MODEL_CATEGORY_INFO[category];
}

/**
 * 对模型列表进行分组
 */
export function groupModelsByCategory<T extends { slug?: string; displayName?: string }>(
  models: T[]
): Record<ModelCategory, T[]> {
  const groups: Record<ModelCategory, T[]> = {
    chatgpt: [],
    claude: [],
    glm: [],
    gemini: [],
    deepseek: [],
    qwen: [],
    grok: [],
    kimi: [],
    other: [],
  };

  for (const model of models) {
    const modelName = model.slug || model.displayName || "";
    const category = classifyModel(modelName);
    groups[category].push(model);
  }

  return groups;
}

/**
 * 获取分类统计
 */
export function getCategoryStats<T extends { slug?: string; displayName?: string }>(
  models: T[]
): Record<ModelCategory, number> {
  const groups = groupModelsByCategory(models);
  const stats: Record<ModelCategory, number> = {
    chatgpt: 0,
    claude: 0,
    glm: 0,
    gemini: 0,
    deepseek: 0,
    qwen: 0,
    grok: 0,
    kimi: 0,
    other: 0,
  };

  for (const [category, items] of Object.entries(groups)) {
    stats[category as ModelCategory] = items.length;
  }

  return stats;
}
