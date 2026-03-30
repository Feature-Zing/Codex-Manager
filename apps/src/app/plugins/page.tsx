"use client";

import { useEffect, useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  Download,
  Info,
  Play,
  RefreshCw,
  Rocket,
  X,
  Trash2,
  ToggleLeft,
  ToggleRight,
} from "lucide-react";
import { toast } from "sonner";
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { ConfirmDialog } from "@/components/modals/confirm-dialog";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Button, buttonVariants } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Skeleton } from "@/components/ui/skeleton";
import { useDesktopPageActive } from "@/hooks/useDesktopPageActive";
import { useDeferredDesktopActivation } from "@/hooks/useDeferredDesktopActivation";
import { usePageTransitionReady } from "@/hooks/usePageTransitionReady";
import { appClient } from "@/lib/api/app-client";
import { pluginClient } from "@/lib/api/plugin-client";
import { useAppStore } from "@/lib/store/useAppStore";
import { cn } from "@/lib/utils";
import {
  InstalledPluginSummary,
  PluginCatalogEntry,
  PluginRunLogSummary,
  PluginTaskSummary,
} from "@/types";

type SelectedPluginDetail =
  | { kind: "catalog"; pluginId: string }
  | { kind: "installed"; pluginId: string }
  | null;

type PluginViewFilter = "installed" | "not-installed" | "update";

const MARKET_MODE_OPTIONS = [
  {
    value: "builtin",
    label: "内置精选",
    description: "默认使用官方精选插件，适合开箱即用。",
  },
  {
    value: "custom",
    label: "自定义源",
    description: "接入你自己的远程 JSON 市场源。",
  },
] as const;

const PLUGIN_VIEW_FILTER_OPTIONS: Array<{
  value: PluginViewFilter;
  label: string;
}> = [
  { value: "installed", label: "已安装" },
  { value: "not-installed", label: "未安装" },
  { value: "update", label: "更新" },
];

function normalizeMarketMode(value: string | null | undefined) {
  return String(value || "")
    .trim()
    .toLowerCase() === "custom"
    ? "custom"
    : "builtin";
}

function formatPermissionLabel(permission: string) {
  switch (permission) {
    case "accounts:cleanup":
      return "清理封禁账号";
    case "settings:read":
      return "读取设置";
    case "network":
      return "网络访问";
    default:
      return permission;
  }
}

function formatMarketCategory(category: string | null | undefined) {
  switch (category) {
    case "official":
      return "官方精选";
    case "private":
      return "企业私有";
    case "community":
      return "社区插件";
    default:
      return category || "";
  }
}

function formatRuntimeKind(runtimeKind: string | null | undefined) {
  switch (runtimeKind) {
    case "rhai":
      return "Rhai";
    case "wasm":
      return "WASM";
    default:
      return runtimeKind || "";
  }
}

function compareVersionStrings(left: string, right: string) {
  const leftParts = left
    .split(/[^0-9]+/)
    .filter(Boolean)
    .map((item) => Number(item));
  const rightParts = right
    .split(/[^0-9]+/)
    .filter(Boolean)
    .map((item) => Number(item));
  const length = Math.max(leftParts.length, rightParts.length);
  for (let index = 0; index < length; index += 1) {
    const leftValue = leftParts[index] ?? 0;
    const rightValue = rightParts[index] ?? 0;
    if (leftValue !== rightValue) {
      return leftValue - rightValue;
    }
  }
  return left.localeCompare(right, undefined, {
    numeric: true,
    sensitivity: "base",
  });
}

function PermissionBadge({ permission }: { permission: string }) {
  return (
    <Badge variant="secondary" className="mr-1.5 mb-1">
      {formatPermissionLabel(permission)}
    </Badge>
  );
}

function StatusBadge({ status }: { status: string }) {
  const normalized = status.toLowerCase();
  const label =
    normalized === "enabled"
      ? "启用中"
      : normalized === "broken"
        ? "异常"
        : "未知";
  const toneClass =
    normalized === "enabled"
      ? "border-emerald-500/20 bg-emerald-500/10 text-emerald-600"
      : normalized === "broken"
        ? "border-red-500/20 bg-red-500/10 text-red-600"
        : "border-amber-500/20 bg-amber-500/10 text-amber-600";
  return <Badge className={toneClass}>{label}</Badge>;
}

function formatDuration(value: number | null): string {
  if (value == null) return "-";
  if (value >= 10_000) return `${Math.round(value / 1000)}s`;
  if (value >= 1000) return `${(value / 1000).toFixed(1).replace(/\.0$/, "")}s`;
  return `${Math.round(value)}ms`;
}

function formatTimestamp(value: number | null): string {
  if (value == null) return "-";
  const date = new Date(value * 1000);
  if (Number.isNaN(date.getTime())) return "-";
  return new Intl.DateTimeFormat("zh-CN", {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
    hour12: false,
  }).format(date);
}

function PluginCard({
  item,
  onOpenDetails,
  onInstall,
}: {
  item: PluginCatalogEntry;
  onOpenDetails: (entry: PluginCatalogEntry) => void;
  onInstall: (entry: PluginCatalogEntry) => void;
}) {
  return (
    <Card className="glass-card border-none shadow-sm">
      <CardHeader className="space-y-2 pb-3">
        <div className="flex items-start justify-between gap-3">
          <div className="min-w-0">
            <CardTitle className="text-base">{item.name}</CardTitle>
            <CardDescription className="mt-1 line-clamp-1">
              {item.description || "暂无描述"}
            </CardDescription>
          </div>
          <Badge variant="secondary">{item.version}</Badge>
        </div>
        <div className="flex flex-wrap gap-2 text-xs text-muted-foreground">
          {item.author ? <span>作者：{item.author}</span> : null}
          <span>权限 {item.permissions.length}</span>
          <span>任务 {item.tasks.length}</span>
          {item.category ? (
            <Badge variant="outline">
              {formatMarketCategory(item.category)}
            </Badge>
          ) : null}
          <Badge variant="outline">{formatRuntimeKind(item.runtimeKind)}</Badge>
        </div>
      </CardHeader>
      <CardContent className="flex items-center justify-between gap-3 pt-0">
        <div className="text-xs text-muted-foreground">
          <span>
            {item.sourceUrl === "builtin://codexmanager"
              ? "来源：内置精选市场"
              : item.sourceUrl
                ? `来源：${item.sourceUrl}`
                : "内置市场"}
          </span>
        </div>
        <div className="flex gap-2">
          <Button
            variant="outline"
            size="sm"
            onClick={() => onOpenDetails(item)}
          >
            <Info className="mr-1.5 h-4 w-4" />
            详情
          </Button>
          <Button size="sm" onClick={() => onInstall(item)} className="gap-2">
            <Download className="h-4 w-4" />
            安装
          </Button>
        </div>
      </CardContent>
    </Card>
  );
}

function InstalledPluginCard({
  item,
  updateVersion,
  onOpenDetails,
  onUpdate,
  onEnable,
  onDisable,
}: {
  item: InstalledPluginSummary;
  updateVersion?: string | null;
  onOpenDetails: (item: InstalledPluginSummary) => void;
  onUpdate?: (pluginId: string) => void;
  onEnable: (pluginId: string) => void;
  onDisable: (pluginId: string) => void;
}) {
  return (
    <Card className="glass-card border-none shadow-sm">
      <CardHeader className="space-y-2 pb-3">
        <div className="flex items-start justify-between gap-3">
          <div className="min-w-0">
            <CardTitle className="text-base">{item.name}</CardTitle>
            <CardDescription className="mt-1 line-clamp-1">
              {item.description || "暂无描述"}
            </CardDescription>
          </div>
          <div className="flex items-center gap-2">
            <Badge variant="secondary">{item.version}</Badge>
            {updateVersion ? (
              <Badge className="border-primary/20 bg-primary/10 text-primary">
                可更新 {updateVersion}
              </Badge>
            ) : null}
            <Badge variant="outline">已安装</Badge>
            <StatusBadge status={item.status} />
          </div>
        </div>
        <div className="flex flex-wrap gap-2 text-xs text-muted-foreground">
          {item.author ? <span>作者：{item.author}</span> : null}
          <span>权限 {item.permissions.length}</span>
          <span>
            任务 {item.enabledTaskCount}/{item.taskCount}
          </span>
          {item.category ? (
            <Badge variant="outline">
              {formatMarketCategory(item.category)}
            </Badge>
          ) : null}
          <Badge variant="outline">{formatRuntimeKind(item.runtimeKind)}</Badge>
        </div>
      </CardHeader>
      <CardContent className="flex items-center justify-between gap-3 pt-0">
        <div className="text-xs text-muted-foreground">
          <span>
            {item.sourceUrl === "builtin://codexmanager"
              ? "来源：内置精选市场"
              : item.sourceUrl
                ? `来源：${item.sourceUrl}`
                : "内置安装"}
          </span>
        </div>
        <div className="flex gap-2">
          <Button
            variant="outline"
            size="sm"
            onClick={() => onOpenDetails(item)}
          >
            <Info className="mr-1.5 h-4 w-4" />
            详情
          </Button>
          {updateVersion && onUpdate ? (
            <Button
              size="sm"
              onClick={() => onUpdate(item.pluginId)}
              className="gap-2"
            >
              <RefreshCw className="h-4 w-4" />
              更新
            </Button>
          ) : item.status === "enabled" ? (
            <Button
              variant="outline"
              size="sm"
              onClick={() => onDisable(item.pluginId)}
            >
              <ToggleLeft className="mr-1.5 h-4 w-4" />
              停用
            </Button>
          ) : (
            <Button
              variant="outline"
              size="sm"
              onClick={() => onEnable(item.pluginId)}
            >
              <ToggleRight className="mr-1.5 h-4 w-4" />
              启用
            </Button>
          )}
        </div>
      </CardContent>
    </Card>
  );
}

export default function PluginsPage() {
  const serviceReady = useAppStore((state) => state.serviceStatus.connected);
  const isPageActive = useDesktopPageActive("/plugins/");
  const isActivationReady = useDeferredDesktopActivation(serviceReady);
  usePageTransitionReady("/plugins/", !serviceReady);
  const queryClient = useQueryClient();
  const [marketMode, setMarketMode] = useState("builtin");
  const [pluginViewFilter, setPluginViewFilter] =
    useState<PluginViewFilter>("not-installed");
  const [sourceUrl, setSourceUrl] = useState("");
  const [selectedPlugin, setSelectedPlugin] =
    useState<SelectedPluginDetail>(null);
  const [pendingUninstallPlugin, setPendingUninstallPlugin] =
    useState<InstalledPluginSummary | null>(null);
  const [taskIntervalDrafts, setTaskIntervalDrafts] = useState<
    Record<string, string>
  >({});

  const settingsQuery = useQuery({
    queryKey: ["plugin-settings"],
    queryFn: () => appClient.getSettings(),
    enabled: isPageActive && isActivationReady,
  });

  useEffect(() => {
    if (settingsQuery.data) {
      setMarketMode(normalizeMarketMode(settingsQuery.data.pluginMarketMode));
      setSourceUrl(settingsQuery.data.pluginMarketSourceUrl || "");
    }
  }, [settingsQuery.data]);

  const catalogQuery = useQuery({
    queryKey: ["plugin-catalog", marketMode, sourceUrl],
    queryFn: () =>
      pluginClient.getCatalog({
        marketMode,
        sourceUrl: marketMode === "custom" ? sourceUrl || undefined : undefined,
      }),
    enabled: isPageActive && isActivationReady,
  });

  const installedQuery = useQuery({
    queryKey: ["plugin-installed"],
    queryFn: () => pluginClient.listInstalled(),
    enabled: isPageActive && isActivationReady,
  });

  const tasksQuery = useQuery({
    queryKey: ["plugin-tasks"],
    queryFn: () => pluginClient.listTasks(),
    enabled: isPageActive && isActivationReady,
  });

  const logsQuery = useQuery({
    queryKey: ["plugin-logs"],
    queryFn: () => pluginClient.listLogs({ limit: 20 }),
    enabled: isPageActive && isActivationReady,
  });

  const saveSourceMutation = useMutation({
    mutationFn: async () =>
      appClient.setSettings({
        pluginMarketMode: normalizeMarketMode(marketMode),
        pluginMarketSourceUrl: sourceUrl,
      }),
    onSuccess: (settings) => {
      setMarketMode(normalizeMarketMode(settings.pluginMarketMode));
      setSourceUrl(settings.pluginMarketSourceUrl || "");
      toast.success("市场源已保存");
      void queryClient.invalidateQueries({ queryKey: ["plugin-catalog"] });
    },
    onError: (error) => {
      toast.error(error instanceof Error ? error.message : "保存市场源失败");
    },
  });

  const installMutation = useMutation({
    mutationFn: (entry: PluginCatalogEntry) => pluginClient.install(entry),
    onSuccess: () => {
      setPluginViewFilter("installed");
      toast.success("插件已安装");
      void queryClient.invalidateQueries({ queryKey: ["plugin-installed"] });
      void queryClient.invalidateQueries({ queryKey: ["plugin-tasks"] });
      void queryClient.invalidateQueries({ queryKey: ["plugin-logs"] });
    },
    onError: (error) => {
      toast.error(error instanceof Error ? error.message : "安装失败");
    },
  });

  const updateMutation = useMutation({
    mutationFn: (payload: { pluginId: string; sourceUrl?: string | null }) =>
      pluginClient.update(payload.pluginId, payload.sourceUrl || undefined),
    onSuccess: () => {
      toast.success("插件已更新");
      void queryClient.invalidateQueries({ queryKey: ["plugin-catalog"] });
      void queryClient.invalidateQueries({ queryKey: ["plugin-installed"] });
      void queryClient.invalidateQueries({ queryKey: ["plugin-tasks"] });
      void queryClient.invalidateQueries({ queryKey: ["plugin-logs"] });
    },
    onError: (error) => {
      toast.error(error instanceof Error ? error.message : "更新失败");
    },
  });

  const toggleMutation = useMutation({
    mutationFn: async (payload: { pluginId: string; enabled: boolean }) =>
      payload.enabled
        ? pluginClient.enable(payload.pluginId)
        : pluginClient.disable(payload.pluginId),
    onSuccess: () => {
      toast.success("插件状态已更新");
      void queryClient.invalidateQueries({ queryKey: ["plugin-installed"] });
    },
    onError: (error) => {
      toast.error(error instanceof Error ? error.message : "更新失败");
    },
  });

  const uninstallMutation = useMutation({
    mutationFn: (pluginId: string) => pluginClient.uninstall(pluginId),
    onSuccess: () => {
      toast.success("插件已卸载");
      void queryClient.invalidateQueries({ queryKey: ["plugin-installed"] });
      void queryClient.invalidateQueries({ queryKey: ["plugin-tasks"] });
      void queryClient.invalidateQueries({ queryKey: ["plugin-logs"] });
    },
    onError: (error) => {
      toast.error(error instanceof Error ? error.message : "卸载失败");
    },
  });

  const runTaskMutation = useMutation({
    mutationFn: (taskId: string) => pluginClient.runTask(taskId),
    onSuccess: () => {
      toast.success("任务已执行");
      void queryClient.invalidateQueries({ queryKey: ["plugin-installed"] });
      void queryClient.invalidateQueries({ queryKey: ["plugin-tasks"] });
      void queryClient.invalidateQueries({ queryKey: ["plugin-logs"] });
    },
    onError: (error) => {
      toast.error(error instanceof Error ? error.message : "运行失败");
    },
  });

  const updateTaskMutation = useMutation({
    mutationFn: (payload: { taskId: string; intervalSeconds: number }) =>
      pluginClient.updateTask(payload.taskId, payload.intervalSeconds),
    onSuccess: () => {
      toast.success("任务间隔已更新");
      void queryClient.invalidateQueries({ queryKey: ["plugin-installed"] });
      void queryClient.invalidateQueries({ queryKey: ["plugin-tasks"] });
      void queryClient.invalidateQueries({ queryKey: ["plugin-logs"] });
    },
    onError: (error) => {
      toast.error(error instanceof Error ? error.message : "更新任务失败");
    },
  });

  const tasksByPluginId = useMemo(() => {
    const map = new Map<string, PluginTaskSummary[]>();
    for (const task of tasksQuery.data || []) {
      const items = map.get(task.pluginId) || [];
      items.push(task);
      map.set(task.pluginId, items);
    }
    return map;
  }, [tasksQuery.data]);

  const logsByPluginId = useMemo(() => {
    const map = new Map<string, PluginRunLogSummary[]>();
    for (const log of logsQuery.data || []) {
      const items = map.get(log.pluginId) || [];
      items.push(log);
      map.set(log.pluginId, items);
    }
    return map;
  }, [logsQuery.data]);

  const catalogItems = catalogQuery.data?.items || [];
  const installedItems = installedQuery.data || [];
  const catalogById = useMemo(
    () => new Map(catalogItems.map((item) => [item.id, item])),
    [catalogItems],
  );
  const installedById = useMemo(
    () => new Map(installedItems.map((item) => [item.pluginId, item])),
    [installedItems],
  );
  const installedPluginIds = useMemo(
    () => new Set(installedItems.map((item) => item.pluginId)),
    [installedItems],
  );
  const updatableVersionByPluginId = useMemo(() => {
    const map = new Map<string, string>();
    for (const item of installedItems) {
      const catalogEntry = catalogById.get(item.pluginId);
      if (
        catalogEntry &&
        compareVersionStrings(catalogEntry.version, item.version) > 0
      ) {
        map.set(item.pluginId, catalogEntry.version);
      }
    }
    return map;
  }, [catalogById, installedItems]);
  const notInstalledCatalogItems = useMemo(
    () => catalogItems.filter((item) => !installedPluginIds.has(item.id)),
    [catalogItems, installedPluginIds],
  );
  const updatableInstalledItems = useMemo(
    () =>
      installedItems.filter((item) => updatableVersionByPluginId.has(item.pluginId)),
    [installedItems, updatableVersionByPluginId],
  );
  const selectedCatalogItem =
    selectedPlugin ? catalogById.get(selectedPlugin.pluginId) || null : null;
  const selectedInstalledItem =
    selectedPlugin ? installedById.get(selectedPlugin.pluginId) || null : null;
  const selectedTasks = selectedPlugin
    ? tasksByPluginId.get(selectedPlugin.pluginId) || []
    : [];
  const selectedLogs = selectedPlugin
    ? logsByPluginId.get(selectedPlugin.pluginId) || []
    : [];
  const selectedDetail = selectedInstalledItem || selectedCatalogItem;
  const selectedUpdateVersion = selectedInstalledItem
    ? updatableVersionByPluginId.get(selectedInstalledItem.pluginId) || null
    : null;

  return (
    <div className="p-6 space-y-6">
      <div className="flex flex-col gap-2">
        <div className="flex items-center gap-3">
          <div className="flex h-10 w-10 items-center justify-center rounded-2xl bg-primary/10 text-primary">
            <Rocket className="h-5 w-5" />
          </div>
          <div>
            <h1 className="text-2xl font-semibold">插件中心</h1>
            <p className="text-sm text-muted-foreground">
              内置精选优先，自定义源按需补充，脚本能力继续由 Rhai 承担。
            </p>
          </div>
        </div>
      </div>

      <Card className="glass-card border-none shadow-sm">
        <CardHeader>
          <CardTitle>市场层</CardTitle>
          <CardDescription>
            只保留内置精选和自定义源两种模式。内置模式完全隔离自定义
            URL，自定义模式才显示并加载远程 JSON 市场。
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="grid gap-3 md:grid-cols-2">
            {MARKET_MODE_OPTIONS.map((option) => (
              <button
                key={option.value}
                type="button"
                onClick={() => setMarketMode(normalizeMarketMode(option.value))}
                className={cn(
                  "rounded-2xl border p-4 text-left transition-all",
                  marketMode === option.value
                    ? "border-primary/40 bg-primary/10 shadow-sm"
                    : "border-border/60 bg-background/40 hover:bg-background/70",
                )}
              >
                <div className="flex items-center justify-between gap-2">
                  <div className="font-medium">{option.label}</div>
                  {marketMode === option.value ? <Badge>已选</Badge> : null}
                </div>
                <div className="mt-1 text-xs leading-5 text-muted-foreground">
                  {option.description}
                </div>
              </button>
            ))}
          </div>
          {marketMode === "custom" ? (
            <>
              <div className="flex flex-col gap-3 md:flex-row md:items-center">
                <Input
                  value={sourceUrl}
                  onChange={(event) => setSourceUrl(event.target.value)}
                  placeholder="https://example.com/plugin-market.json"
                  className="md:flex-1"
                />
                <div className="flex gap-2">
                  <Button
                    onClick={() => saveSourceMutation.mutate()}
                    disabled={saveSourceMutation.isPending}
                  >
                    保存
                  </Button>
                  <Button
                    variant="outline"
                    onClick={() =>
                      void queryClient.invalidateQueries({
                        queryKey: ["plugin-catalog"],
                      })
                    }
                  >
                    <RefreshCw className="mr-2 h-4 w-4" />
                    刷新
                  </Button>
                </div>
              </div>
              <div className="rounded-2xl border border-dashed border-border/60 bg-muted/20 p-4 text-xs text-muted-foreground">
                {catalogQuery.data?.sourceUrl
                  ? `当前使用自定义源：${catalogQuery.data.sourceUrl}`
                  : "当前使用自定义源，适合接入你自己的 JSON 市场文件。"}
              </div>
            </>
          ) : (
            <div className="rounded-2xl border border-dashed border-border/60 bg-muted/20 p-4 text-xs text-muted-foreground">
              当前使用内置精选市场，默认只显示官方内置脚本插件。
            </div>
          )}
        </CardContent>
      </Card>

      <Card className="glass-card border-none shadow-sm">
        <CardHeader className="space-y-4">
          <div>
            <CardTitle>插件列表</CardTitle>
            <CardDescription>
              一个面板统一查看插件。未安装看当前市场，已安装看本地插件，更新只显示当前市场里有新版本的已安装插件。
            </CardDescription>
          </div>
          <div className="flex flex-wrap gap-2">
            {PLUGIN_VIEW_FILTER_OPTIONS.map((option) => {
              const count =
                option.value === "installed"
                  ? installedItems.length
                  : option.value === "update"
                    ? updatableInstalledItems.length
                    : notInstalledCatalogItems.length;
              return (
                <button
                  key={option.value}
                  type="button"
                  onClick={() => setPluginViewFilter(option.value)}
                  className={cn(
                    "flex items-center gap-2 rounded-full border px-4 py-2 text-sm transition-all",
                    pluginViewFilter === option.value
                      ? "border-primary/40 bg-primary/10 text-primary shadow-sm"
                      : "border-border/60 bg-background/40 text-muted-foreground hover:bg-background/70",
                  )}
                >
                  <span>{option.label}</span>
                  <Badge variant="secondary">{count}</Badge>
                </button>
              );
            })}
          </div>
        </CardHeader>
        <CardContent>
          {catalogQuery.isLoading || installedQuery.isLoading ? (
            <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-3">
              {Array.from({ length: 2 }).map((_, index) => (
                <Skeleton key={index} className="h-72 rounded-2xl" />
              ))}
            </div>
          ) : pluginViewFilter === "installed" ? (
            installedItems.length > 0 ? (
              <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-3">
                {installedItems.map((item) => (
                  <InstalledPluginCard
                    key={item.pluginId}
                    item={item}
                    updateVersion={
                      updatableVersionByPluginId.get(item.pluginId) || null
                    }
                    onOpenDetails={(entry) =>
                      setSelectedPlugin({
                        kind: "installed",
                        pluginId: entry.pluginId,
                      })
                    }
                    onUpdate={(pluginId) =>
                      updateMutation.mutate({
                        pluginId,
                        sourceUrl:
                          catalogById.get(pluginId)?.sourceUrl || item.sourceUrl,
                      })
                    }
                    onEnable={(pluginId) =>
                      toggleMutation.mutate({ pluginId, enabled: true })
                    }
                    onDisable={(pluginId) =>
                      toggleMutation.mutate({ pluginId, enabled: false })
                    }
                  />
                ))}
              </div>
            ) : (
              <div className="rounded-2xl border border-dashed border-border/60 p-10 text-center text-sm text-muted-foreground">
                还没有安装任何插件
              </div>
            )
          ) : pluginViewFilter === "update" ? (
            updatableInstalledItems.length > 0 ? (
              <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-3">
                {updatableInstalledItems.map((item) => (
                  <InstalledPluginCard
                    key={item.pluginId}
                    item={item}
                    updateVersion={
                      updatableVersionByPluginId.get(item.pluginId) || null
                    }
                    onOpenDetails={(entry) =>
                      setSelectedPlugin({
                        kind: "installed",
                        pluginId: entry.pluginId,
                      })
                    }
                    onUpdate={(pluginId) =>
                      updateMutation.mutate({
                        pluginId,
                        sourceUrl:
                          catalogById.get(pluginId)?.sourceUrl || item.sourceUrl,
                      })
                    }
                    onEnable={(pluginId) =>
                      toggleMutation.mutate({ pluginId, enabled: true })
                    }
                    onDisable={(pluginId) =>
                      toggleMutation.mutate({ pluginId, enabled: false })
                    }
                  />
                ))}
              </div>
            ) : (
              <div className="rounded-2xl border border-dashed border-border/60 p-10 text-center text-sm text-muted-foreground">
                当前市场没有可更新插件
              </div>
            )
          ) : notInstalledCatalogItems.length > 0 ? (
            <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-3">
              {notInstalledCatalogItems.map((item) => (
                <PluginCard
                  key={item.id}
                  item={item}
                  onOpenDetails={(entry) =>
                    setSelectedPlugin({ kind: "catalog", pluginId: entry.id })
                  }
                  onInstall={(entry) => installMutation.mutate(entry)}
                />
              ))}
            </div>
          ) : (
            <div className="rounded-2xl border border-dashed border-border/60 p-10 text-center text-sm text-muted-foreground">
              {marketMode === "custom" && !catalogQuery.data?.sourceUrl
                ? "当前还没有配置自定义源，所以这里不会显示未安装插件。"
                : "暂无未安装插件"}
            </div>
          )}
        </CardContent>
      </Card>

      <Dialog
        open={selectedPlugin !== null}
        onOpenChange={(open) => !open && setSelectedPlugin(null)}
      >
        <DialogContent
          showCloseButton={false}
          className="glass-card max-h-[85vh] overflow-hidden border-none p-0 sm:max-w-[860px] lg:max-w-[920px]"
        >
          {selectedDetail ? (
            <div className="flex max-h-[85vh] flex-col">
              <div className="shrink-0 bg-muted/20 px-6 pt-6">
                <div className="flex items-start justify-between gap-4">
                  <DialogHeader className="mb-4 min-w-0 flex-1">
                    <div className="flex flex-wrap items-center gap-2">
                      <DialogTitle className="text-xl">
                        {selectedDetail.name}
                      </DialogTitle>
                      <Badge variant="secondary">
                        {selectedDetail.version}
                      </Badge>
                      {"status" in selectedDetail ? (
                        <StatusBadge status={selectedDetail.status} />
                      ) : null}
                    </div>
                    <DialogDescription className="break-words text-sm">
                      {selectedDetail.description || "暂无描述"}
                    </DialogDescription>
                    <div className="flex flex-wrap gap-2 text-xs text-muted-foreground">
                      {selectedDetail.author ? (
                        <span>作者：{selectedDetail.author}</span>
                      ) : null}
                      {selectedDetail.sourceUrl ? (
                        <span>
                          来源：
                          {selectedDetail.sourceUrl === "builtin://codexmanager"
                            ? "内置精选市场"
                            : selectedDetail.sourceUrl}
                        </span>
                      ) : null}
                      <span>权限 {selectedDetail.permissions.length}</span>
                      {"taskCount" in selectedDetail ? (
                        <span>
                          任务 {selectedDetail.enabledTaskCount}/
                          {selectedDetail.taskCount}
                        </span>
                      ) : (
                        <span>任务 {selectedDetail.tasks.length}</span>
                      )}
                    </div>
                    <div className="flex flex-wrap gap-2 text-xs text-muted-foreground">
                      <span>清单版本 {selectedDetail.manifestVersion}</span>
                      <span>
                        运行时 {formatRuntimeKind(selectedDetail.runtimeKind)}
                      </span>
                      {selectedDetail.category ? (
                        <span>
                          分类 {formatMarketCategory(selectedDetail.category)}
                        </span>
                      ) : null}
                      {selectedDetail.tags.length > 0 ? (
                        <span>标签 {selectedDetail.tags.join(" / ")}</span>
                      ) : null}
                    </div>
                  </DialogHeader>
                  <DialogClose
                    className={cn(
                      buttonVariants({ variant: "ghost", size: "icon-sm" }),
                      "shrink-0 text-muted-foreground hover:bg-muted hover:text-foreground",
                    )}
                    type="button"
                  >
                    <X className="h-4 w-4" />
                    <span className="sr-only">关闭</span>
                  </DialogClose>
                </div>
              </div>

              <div className="max-h-[calc(85vh-154px)] overflow-y-auto px-6 py-6">
                <div className="grid gap-4">
                  <div className="rounded-2xl border border-border/60 bg-background/60 p-4">
                    <div className="mb-2 text-sm font-medium">权限</div>
                    <div>
                      {selectedDetail.permissions.length > 0 ? (
                        selectedDetail.permissions.map((permission) => (
                          <PermissionBadge
                            key={permission}
                            permission={permission}
                          />
                        ))
                      ) : (
                        <div className="text-sm text-muted-foreground">
                          无需额外权限
                        </div>
                      )}
                    </div>
                  </div>

                  <div className="rounded-2xl border border-border/60 bg-background/60 p-4">
                    <div className="mb-2 text-sm font-medium">任务</div>
                    <div className="space-y-2">
                      {selectedTasks.length > 0 ? (
                        selectedTasks.map((task) => (
                          <div
                            key={task.id}
                            className="rounded-xl border border-border/60 bg-background p-3 text-sm"
                          >
                            <div className="flex items-start justify-between gap-3">
                              <div className="min-w-0">
                                <div className="font-medium">{task.name}</div>
                                <div className="mt-1 break-words text-xs text-muted-foreground">
                                  {task.scheduleKind === "manual"
                                    ? "手动"
                                    : `每 ${task.intervalSeconds || 0} 秒`}
                                  {" · "}
                                  {task.entrypoint}
                                </div>
                              </div>
                              <div className="flex items-center gap-2">
                                <Badge variant="outline">
                                  {task.enabled ? "启用" : "禁用"}
                                </Badge>
                                {selectedPlugin?.kind === "installed" ? (
                                  <Button
                                    size="sm"
                                    variant="secondary"
                                    onClick={() =>
                                      runTaskMutation.mutate(task.id)
                                    }
                                  >
                                    <Play className="mr-1.5 h-3.5 w-3.5" />
                                    运行
                                  </Button>
                                ) : null}
                              </div>
                            </div>
                            {task.description ? (
                              <div className="mt-1 break-words text-xs text-muted-foreground">
                                {task.scheduleKind === "manual"
                                  ? task.description
                                  : `每 ${task.intervalSeconds || 0} 秒自动执行一次。`}
                              </div>
                            ) : null}
                            {task.lastError ? (
                              <div className="mt-1 break-words text-xs text-red-500">
                                {task.lastError}
                              </div>
                            ) : null}
                            {"scheduleKind" in task &&
                            task.scheduleKind !== "manual" ? (
                              <div className="mt-3 grid gap-2 rounded-xl border border-border/60 bg-background/70 p-3">
                                <div className="text-xs font-medium text-muted-foreground">
                                  自动执行间隔
                                </div>
                                <div className="flex flex-col gap-2 sm:flex-row sm:items-center">
                                  <Input
                                    type="number"
                                    min={1}
                                    step={1}
                                    className="h-9 w-full sm:max-w-[180px]"
                                    value={
                                      taskIntervalDrafts[task.id] ??
                                      String(task.intervalSeconds || 60)
                                    }
                                    onChange={(event) =>
                                      setTaskIntervalDrafts((prev) => ({
                                        ...prev,
                                        [task.id]: event.target.value,
                                      }))
                                    }
                                    disabled={updateTaskMutation.isPending}
                                  />
                                  <span className="text-xs text-muted-foreground">
                                    秒
                                  </span>
                                  <Button
                                    size="sm"
                                    variant="outline"
                                    className="sm:ml-auto"
                                    disabled={updateTaskMutation.isPending}
                                    onClick={() => {
                                      const raw =
                                        taskIntervalDrafts[task.id] ??
                                        String(task.intervalSeconds || 60);
                                      const intervalSeconds = Number(raw);
                                      if (
                                        !Number.isFinite(intervalSeconds) ||
                                        intervalSeconds <= 0
                                      ) {
                                        toast.error("请输入大于 0 的秒数");
                                        return;
                                      }
                                      updateTaskMutation.mutate({
                                        taskId: task.id,
                                        intervalSeconds:
                                          Math.floor(intervalSeconds),
                                      });
                                    }}
                                  >
                                    保存
                                  </Button>
                                </div>
                                <div className="break-words text-[11px] text-muted-foreground">
                                  当前设置为每 {task.intervalSeconds || 0}{" "}
                                  秒自动执行一次。
                                </div>
                              </div>
                            ) : null}
                          </div>
                        ))
                      ) : (
                        <div className="text-sm text-muted-foreground">
                          暂无任务
                        </div>
                      )}
                    </div>
                  </div>

                  {selectedPlugin?.kind === "installed" ? (
                    <div className="rounded-2xl border border-border/60 bg-background/60 p-4">
                      <div className="mb-2 text-sm font-medium">最近运行</div>
                      <div className="space-y-2">
                        {selectedLogs.length > 0 ? (
                          selectedLogs.slice(0, 5).map((log) => (
                            <div
                              key={log.id}
                              className={cn(
                                "rounded-xl border p-3 text-xs",
                                log.status === "ok"
                                  ? "border-emerald-500/20 bg-emerald-500/5"
                                  : "border-red-500/20 bg-red-500/5",
                              )}
                            >
                              <div className="flex items-center justify-between gap-2">
                                <div className="font-medium">
                                  {log.taskName || log.taskId || "未知任务"}
                                </div>
                                <Badge
                                  variant={
                                    log.status === "ok"
                                      ? "secondary"
                                      : "destructive"
                                  }
                                >
                                  {log.status}
                                </Badge>
                              </div>
                              <div className="mt-1 break-words text-muted-foreground">
                                {log.error ||
                                  (log.output
                                    ? JSON.stringify(log.output)
                                    : "无输出")}
                              </div>
                              <div className="mt-2 flex flex-wrap gap-x-3 gap-y-1 text-[11px] text-muted-foreground">
                                <span>
                                  执行于 {formatTimestamp(log.startedAt)}
                                </span>
                                <span>
                                  耗时 {formatDuration(log.durationMs)}
                                </span>
                              </div>
                            </div>
                          ))
                        ) : (
                          <div className="text-sm text-muted-foreground">
                            暂无日志
                          </div>
                        )}
                      </div>
                    </div>
                  ) : null}
                </div>
              </div>

              <DialogFooter className="mx-0 mb-0 rounded-b-xl border-t border-border/60 bg-muted/20 px-6 py-4 sm:items-center sm:justify-end">
                {selectedPlugin?.kind === "catalog" && selectedCatalogItem ? (
                  <Button
                    className="gap-2"
                    onClick={() => {
                      installMutation.mutate(selectedCatalogItem);
                      setSelectedPlugin(null);
                    }}
                  >
                    <Download className="h-4 w-4" />
                    安装
                  </Button>
                ) : null}
                {selectedPlugin?.kind === "installed" &&
                selectedInstalledItem ? (
                  <>
                    {selectedUpdateVersion ? (
                      <Button
                        className="gap-2"
                        onClick={() =>
                          updateMutation.mutate({
                            pluginId: selectedInstalledItem.pluginId,
                            sourceUrl:
                              selectedCatalogItem?.sourceUrl ||
                              selectedInstalledItem.sourceUrl,
                          })
                        }
                      >
                        <RefreshCw className="h-4 w-4" />
                        更新到 {selectedUpdateVersion}
                      </Button>
                    ) : null}
                    {selectedInstalledItem.status === "enabled" ? (
                      <Button
                        variant="outline"
                        className="gap-2"
                        onClick={() =>
                          toggleMutation.mutate({
                            pluginId: selectedInstalledItem.pluginId,
                            enabled: false,
                          })
                        }
                      >
                        <ToggleLeft className="h-4 w-4" />
                        停用
                      </Button>
                    ) : (
                      <Button
                        variant="outline"
                        className="gap-2"
                        onClick={() =>
                          toggleMutation.mutate({
                            pluginId: selectedInstalledItem.pluginId,
                            enabled: true,
                          })
                        }
                      >
                        <ToggleRight className="h-4 w-4" />
                        启用
                      </Button>
                    )}
                    <Button
                      variant="destructive"
                      className="gap-2"
                      onClick={() =>
                        setPendingUninstallPlugin(selectedInstalledItem)
                      }
                    >
                      <Trash2 className="h-4 w-4" />
                      卸载
                    </Button>
                  </>
                ) : null}
              </DialogFooter>
            </div>
          ) : null}
        </DialogContent>
      </Dialog>

      <ConfirmDialog
        open={pendingUninstallPlugin !== null}
        onOpenChange={(open) => {
          if (!open) {
            setPendingUninstallPlugin(null);
          }
        }}
        title="卸载插件"
        description={
          pendingUninstallPlugin
            ? `确认卸载插件「${pendingUninstallPlugin.name}」吗？卸载后对应任务和运行记录会一并清理。`
            : "确认卸载这个插件吗？"
        }
        confirmText="卸载"
        confirmVariant="destructive"
        onConfirm={() => {
          if (!pendingUninstallPlugin) {
            return;
          }
          uninstallMutation.mutate(pendingUninstallPlugin.pluginId);
          setSelectedPlugin(null);
          setPendingUninstallPlugin(null);
        }}
      />
    </div>
  );
}
