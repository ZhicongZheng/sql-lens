import {
  createContext,
  useCallback,
  useContext,
  useMemo,
  useState,
  type ReactNode,
} from "react";

interface DetailDrawerContextValue {
  isOpen: boolean;
  selectedEventId: string | null;
  openDrawer: (eventId?: string) => void;
  closeDrawer: () => void;
}

const DetailDrawerContext = createContext<DetailDrawerContextValue | undefined>(
  undefined,
);

export function DetailDrawerProvider({ children }: { children: ReactNode }) {
  const [isOpen, setIsOpen] = useState(false);
  const [selectedEventId, setSelectedEventId] = useState<string | null>(null);

  const openDrawer = useCallback((eventId?: string) => {
    setSelectedEventId(eventId ?? null);
    setIsOpen(true);
  }, []);

  const closeDrawer = useCallback(() => {
    setSelectedEventId(null);
    setIsOpen(false);
  }, []);

  const value = useMemo<DetailDrawerContextValue>(
    () => ({ isOpen, selectedEventId, openDrawer, closeDrawer }),
    [isOpen, selectedEventId, openDrawer, closeDrawer],
  );

  return (
    <DetailDrawerContext.Provider value={value}>
      {children}
    </DetailDrawerContext.Provider>
  );
}

export function useDetailDrawer(): DetailDrawerContextValue {
  const ctx = useContext(DetailDrawerContext);
  if (!ctx)
    throw new Error(
      "useDetailDrawer must be used within a DetailDrawerProvider",
    );
  return ctx;
}
