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
  openDrawer: () => void;
  closeDrawer: () => void;
}

const DetailDrawerContext = createContext<DetailDrawerContextValue | undefined>(
  undefined,
);

export function DetailDrawerProvider({ children }: { children: ReactNode }) {
  const [isOpen, setIsOpen] = useState(false);

  const openDrawer = useCallback(() => setIsOpen(true), []);
  const closeDrawer = useCallback(() => setIsOpen(false), []);

  const value = useMemo<DetailDrawerContextValue>(
    () => ({ isOpen, openDrawer, closeDrawer }),
    [isOpen, openDrawer, closeDrawer],
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
