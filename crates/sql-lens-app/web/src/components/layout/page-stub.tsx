interface PageStubProps {
  title: string;
  description?: string;
}

// Placeholder page used by the skeleton's route stubs. Real feature
// implementations land in later issues; this keeps the shell shippable.
export function PageStub({ title, description }: PageStubProps) {
  return (
    <div className="space-y-3">
      <h1 className="text-2xl font-semibold tracking-tight">{title}</h1>
      {description ? (
        <p className="text-sm text-muted-foreground">{description}</p>
      ) : null}
      <p className="text-sm text-muted-foreground">
        This view is a skeleton placeholder. Implementation lands in a follow-up
        issue.
      </p>
    </div>
  );
}
