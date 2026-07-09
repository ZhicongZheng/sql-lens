import { useDetailDrawer } from "@/app/providers/detail-drawer-provider";
import { SqlDetail } from "@/components/sql/sql-detail";
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
} from "@/components/ui/sheet";

export function DetailDrawer() {
  const { isOpen, selectedEventId, closeDrawer } = useDetailDrawer();

  return (
    <Sheet open={isOpen} onOpenChange={(open) => !open && closeDrawer()}>
      <SheetContent
        side="right"
        className="flex w-full flex-col sm:max-w-lg"
        aria-describedby="detail-drawer-description"
      >
        <SheetHeader>
          <SheetTitle>
            {selectedEventId ? "SQL Detail" : "Detail"}
          </SheetTitle>
          <SheetDescription id="detail-drawer-description">
            {selectedEventId
              ? `Event ${selectedEventId}`
              : "Select an event or connection to view details."}
          </SheetDescription>
        </SheetHeader>
        {selectedEventId ? (
          <SqlDetail eventId={selectedEventId} />
        ) : (
          <div className="flex flex-1 items-center justify-center p-6 text-sm text-muted-foreground">
            Select an event or connection to view details.
          </div>
        )}
      </SheetContent>
    </Sheet>
  );
}
