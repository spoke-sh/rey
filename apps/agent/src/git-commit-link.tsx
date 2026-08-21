import type { ReactNode } from "react";
import { shortDigest, sourceCommitUrl } from "./domain";

export interface GitCommitLinkProps {
  children?: ReactNode;
  className?: string;
  fallback?: ReactNode;
  repository: string | null | undefined;
  revision: string;
  title?: string;
}

export function GitCommitLink({
  children,
  className,
  fallback = "GIT COMMIT LINK UNAVAILABLE",
  repository,
  revision,
  title = `Open Git commit ${revision}`,
}: GitCommitLinkProps) {
  const href = repository ? sourceCommitUrl(repository, revision) : null;
  if (!href) return <span className={className}>{fallback}</span>;
  return (
    <a
      className={className}
      data-git-sha={revision}
      href={href}
      rel="noreferrer"
      target="_blank"
      title={title}
    >
      {children ?? shortDigest(revision)}
    </a>
  );
}
