import { useDetailDrawer } from "@/app/providers/detail-drawer-provider";
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
} from "@/components/ui/sheet";

export function DetailDrawer() {
  const { isOpen, closeDrawer } = useDetailDrawer();

  return (
    <Sheet open={isOpen} onOpenChange={(open) => !open && closeDrawer()}>
      <SheetContent
        side="right"
        className="w-full sm:max-w-lg"
        aria-describedby="detail-drawer-description"
      >
        <SheetHeader>
          <SheetTitle>Detail</SheetTitle>
          <SheetDescription id="detail-drawer-description">
            Detail panel — content arrives with SQL Detail / Connection Detail
            features.
          </SheetDescription>
        </SheetHeader>
        <div className="flex flex-1 items-center justify-center p-6 text-sm text-muted-foreground">
          Select an event or connection to view details.
        </div>
      </SheetContent>
    </Sheet>
  );
}
