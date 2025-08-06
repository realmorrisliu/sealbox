import { Skeleton } from "@/components/ui/skeleton";
import { Card } from "@/components/ui/card";

// Skeleton screen for secrets list
export function SecretsListSkeleton() {
  return (
    <div className="space-y-6">
      {/* Title and button skeleton */}
      <div className="flex items-start justify-between">
        <div className="space-y-2">
          <Skeleton className="h-10 w-48" />
          <Skeleton className="h-4 w-64" />
        </div>
        <Skeleton className="h-10 w-40" />
      </div>

      {/* Content card skeleton */}
      <Card className="bg-card border border-border">
        {/* Mobile view skeleton */}
        <div className="block md:hidden space-y-4 p-6">
          {Array.from({ length: 3 }).map((_, i) => (
            <div
              key={i}
              className="bg-card border border-border rounded-md p-4 space-y-4"
            >
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

        {/* Desktop table skeleton */}
        <div className="hidden md:block overflow-x-auto">
          <div className="p-6">
            {/* Table header skeleton */}
            <div className="flex items-center space-x-4 pb-4 border-b border-border">
              <Skeleton className="h-5 w-24" />
              <Skeleton className="h-5 w-16" />
              <Skeleton className="h-5 w-20 hidden lg:block" />
              <Skeleton className="h-5 w-20 hidden xl:block" />
              <Skeleton className="h-5 w-20" />
              <div className="flex-1" />
              <Skeleton className="h-5 w-16" />
            </div>

            {/* Table row skeleton */}
            {Array.from({ length: 5 }).map((_, i) => (
              <div
                key={i}
                className="flex items-center space-x-4 py-4 border-b border-border last:border-b-0"
              >
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

// General page loading skeleton
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

// Form loading skeleton
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

// Card list skeleton
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

// Master key list skeleton
export function MasterKeyListSkeleton() {
  return (
    <div className="space-y-6">
      {/* Page title skeleton */}
      <div className="flex items-start justify-between">
        <div className="space-y-2">
          <Skeleton className="h-10 w-40" />
          <Skeleton className="h-4 w-80" />
        </div>
        <div className="flex items-center space-x-2">
          <Skeleton className="h-10 w-24" />
          <Skeleton className="h-10 w-28" />
        </div>
      </div>

      {/* Content card skeleton */}
      <Card className="bg-card border border-border">
        {/* Mobile view skeleton */}
        <div className="block md:hidden space-y-4 p-6">
          {Array.from({ length: 2 }).map((_, i) => (
            <div
              key={i}
              className="bg-card border border-border rounded-md p-4 space-y-4"
            >
              <div className="flex items-center justify-between">
                <Skeleton className="h-5 w-28" />
                <Skeleton className="h-6 w-16" />
              </div>
              <div className="space-y-2">
                <div className="flex justify-between">
                  <Skeleton className="h-4 w-16" />
                  <Skeleton className="h-4 w-32" />
                </div>
                <div className="flex justify-between items-start">
                  <Skeleton className="h-4 w-20" />
                  <Skeleton className="h-4 w-24" />
                </div>
              </div>
            </div>
          ))}
        </div>

        {/* Desktop table skeleton */}
        <div className="hidden md:block overflow-x-auto">
          <div className="p-6">
            {/* Table header skeleton */}
            <div className="flex items-center space-x-4 pb-4 border-b border-border">
              <Skeleton className="h-5 w-20" />
              <Skeleton className="h-5 w-16" />
              <Skeleton className="h-5 w-20 hidden lg:block" />
              <Skeleton className="h-5 w-24 hidden xl:block" />
              <div className="flex-1" />
              <Skeleton className="h-5 w-16" />
            </div>

            {/* Table row skeleton */}
            {Array.from({ length: 3 }).map((_, i) => (
              <div
                key={i}
                className="flex items-center space-x-4 py-4 border-b border-border last:border-b-0"
              >
                <Skeleton className="h-4 w-36" />
                <Skeleton className="h-6 w-16" />
                <Skeleton className="h-4 w-32 hidden lg:block" />
                <Skeleton className="h-4 w-40 hidden xl:block" />
                <div className="flex-1" />
                <div className="flex items-center justify-end">
                  <Skeleton className="h-8 w-8" />
                </div>
              </div>
            ))}
          </div>
        </div>
      </Card>

      {/* Info alert skeleton */}
      <div className="flex items-start space-x-3 p-4 border border-border rounded-md">
        <Skeleton className="h-4 w-4 mt-0.5" />
        <div className="space-y-2 flex-1">
          <Skeleton className="h-4 w-32" />
          <Skeleton className="h-3 w-full" />
        </div>
      </div>
    </div>
  );
}
