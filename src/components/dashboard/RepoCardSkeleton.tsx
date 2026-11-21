import { Card, CardBody } from "@/components/primitives/Card";
import { Skeleton } from "@/components/primitives/Skeleton";

export function RepoCardSkeleton() {
  return (
    <Card>
      <CardBody className="space-y-3 px-4 pb-3 pt-4">
        <div className="flex items-center gap-2">
          <Skeleton className="h-3.5 w-32" />
          <Skeleton className="ml-auto h-4 w-16 rounded-full" />
        </div>
        <Skeleton className="h-3 w-40" />
        <div className="flex items-center gap-2 pt-2">
          <Skeleton className="h-3 w-20" />
          <Skeleton className="ml-auto h-6 w-24 rounded-control" />
        </div>
      </CardBody>
    </Card>
  );
}

export function RepoCardSkeletonGrid({ count = 6 }: { count?: number }) {
  return (
    <div className="grid gap-[18px] [grid-template-columns:repeat(auto-fill,minmax(320px,1fr))]">
      {Array.from({ length: count }, (_, index) => (
        <RepoCardSkeleton key={index} />
      ))}
    </div>
  );
}
