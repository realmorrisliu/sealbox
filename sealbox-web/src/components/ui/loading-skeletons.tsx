import { Skeleton } from "@/components/ui/skeleton";
import { Card } from "@/components/ui/card";

// 密钥列表的骨架屏
export function SecretsListSkeleton() {
  return (
    <div className="space-section">
      {/* 标题和按钮骨架 */}
      <div className="flex items-start justify-between">
        <div className="space-tight">
          <Skeleton className="h-10 w-48" />
          <Skeleton className="h-4 w-64" />
        </div>
        <Skeleton className="h-10 w-40" />
      </div>

      {/* 内容卡片骨架 */}
      <Card className="bg-card border border-border">
        {/* 移动端视图骨架 */}
        <div className="block md:hidden space-y-4 p-6">
          {Array.from({ length: 3 }).map((_, i) => (
            <div key={i} className="bg-card border border-border rounded-md p-4 space-y-4">
              <div className="flex items-center justify-between">
                <Skeleton className="h-5 w-32" />
                <Skeleton className="h-6 w-12" />
              </div>
              <div className="space-y-2">
                <div className="flex justify-between">
                  <Skeleton className="h-4 w-16" />
                  <Skeleton className="h-4 w-32" />
                </div>
                <div className="flex justify-between items-center">
                  <Skeleton className="h-4 w-16" />
                  <Skeleton className="h-6 w-20" />
                </div>
              </div>
              <div className="flex items-center justify-end space-x-2 pt-2 border-t border-border">
                <Skeleton className="h-8 w-16" />
                <Skeleton className="h-8 w-16" />
              </div>
            </div>
          ))}
        </div>

        {/* 桌面端表格骨架 */}
        <div className="hidden md:block overflow-x-auto">
          <div className="p-6">
            {/* 表头骨架 */}
            <div className="flex items-center space-x-4 pb-4 border-b border-border">
              <Skeleton className="h-5 w-24" />
              <Skeleton className="h-5 w-16" />
              <Skeleton className="h-5 w-20 hidden lg:block" />
              <Skeleton className="h-5 w-20 hidden xl:block" />
              <Skeleton className="h-5 w-20" />
              <div className="flex-1" />
              <Skeleton className="h-5 w-16" />
            </div>
            
            {/* 表格行骨架 */}
            {Array.from({ length: 5 }).map((_, i) => (
              <div key={i} className="flex items-center space-x-4 py-4 border-b border-border last:border-b-0">
                <Skeleton className="h-4 w-32" />
                <Skeleton className="h-6 w-12" />
                <Skeleton className="h-4 w-32 hidden lg:block" />
                <Skeleton className="h-4 w-32 hidden xl:block" />
                <Skeleton className="h-6 w-20" />
                <div className="flex-1" />
                <div className="flex items-center space-x-2">
                  <Skeleton className="h-8 w-8" />
                  <Skeleton className="h-8 w-8" />
                </div>
              </div>
            ))}
          </div>
        </div>
      </Card>
    </div>
  );
}

// 通用页面加载骨架
export function PageLoadingSkeleton() {
  return (
    <div className="flex items-center justify-center h-64">
      <div className="text-center space-y-4">
        <Skeleton className="h-8 w-8 rounded-full mx-auto" />
        <Skeleton className="h-4 w-24 mx-auto" />
      </div>
    </div>
  );
}

// 表单加载骨架
export function FormSkeleton() {
  return (
    <div className="space-y-6">
      <div className="space-y-2">
        <Skeleton className="h-4 w-20" />
        <Skeleton className="h-10 w-full" />
      </div>
      <div className="space-y-2">
        <Skeleton className="h-4 w-24" />
        <Skeleton className="h-10 w-full" />
      </div>
      <div className="space-y-2">
        <Skeleton className="h-4 w-16" />
        <Skeleton className="h-20 w-full" />
      </div>
      <div className="flex justify-end space-x-2">
        <Skeleton className="h-10 w-20" />
        <Skeleton className="h-10 w-24" />
      </div>
    </div>
  );
}

// 卡片列表骨架
export function CardListSkeleton({ count = 3 }: { count?: number }) {
  return (
    <div className="space-y-4">
      {Array.from({ length: count }).map((_, i) => (
        <Card key={i} className="p-6">
          <div className="flex items-start space-x-4">
            <Skeleton className="h-12 w-12 rounded-full" />
            <div className="space-y-2 flex-1">
              <Skeleton className="h-5 w-3/4" />
              <Skeleton className="h-4 w-1/2" />
              <Skeleton className="h-4 w-2/3" />
            </div>
            <Skeleton className="h-8 w-20" />
          </div>
        </Card>
      ))}
    </div>
  );
}