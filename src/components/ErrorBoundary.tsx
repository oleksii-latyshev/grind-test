import { Component, type ErrorInfo, type ReactNode } from "react";
import { RotateCcw, TriangleAlert } from "lucide-react";

import { useT } from "@/i18n";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";

interface Props {
  children: ReactNode;
  onReset: () => void;
}

// A class component cannot use hooks, and only a class can be an error boundary — so the
// translated text lives in two small function components instead.
function ErrorText() {
  const t = useT();
  return (
    <>
      <h2 className="text-sm font-semibold">{t("error.title")}</h2>
      <p className="text-sm leading-relaxed text-muted-foreground">{t("error.body")}</p>
    </>
  );
}

function ErrorHomeLabel() {
  return <>{useT()("error.home")}</>;
}

interface State {
  error: Error | null;
}

/**
 * Without this, one bad render unmounts the whole tree and leaves a blank window with no
 * explanation and no way back — which is exactly what a crash in the middle of a study
 * session used to look like.
 */
export class ErrorBoundary extends Component<Props, State> {
  state: State = { error: null };

  static getDerivedStateFromError(error: Error): State {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error("render error", error, info.componentStack);
  }

  render() {
    const { error } = this.state;
    if (!error) return this.props.children;

    return (
      <div className="mx-auto w-full max-w-2xl p-8">
        <Card className="border-destructive/40">
          <CardContent className="space-y-4 py-6">
            <div className="flex items-start gap-3">
              <TriangleAlert className="mt-0.5 size-5 shrink-0 text-destructive" />
              <div className="space-y-1">
                <ErrorText />
              </div>
            </div>

            <pre className="overflow-x-auto rounded-4xl border border-border bg-muted/50 p-4 font-mono text-xs">
              {error.message}
            </pre>

            <Button
              onClick={() => {
                this.setState({ error: null });
                this.props.onReset();
              }}
            >
              <RotateCcw className="size-4" />
              <ErrorHomeLabel />
            </Button>
          </CardContent>
        </Card>
      </div>
    );
  }
}
