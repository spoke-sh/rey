import * as stylex from "@stylexjs/stylex";

const mono = 'var(--mono, "SFMono-Regular", Consolas, monospace)';
const display =
  'var(--display, "Arial Narrow", "Roboto Condensed", sans-serif)';

export const workloadsStyles = stylex.create({
  page: { paddingTop: "clamp(46px, 6vw, 88px)" },
  section: { marginTop: "clamp(74px, 9vw, 126px)" },
  firstSection: { marginTop: 0 },
  portfolioLink: {
    color: "inherit",
    display: "inline-block",
    fontFamily: mono,
    fontSize: "0.64rem",
    letterSpacing: "0.08em",
    marginBottom: 54,
    textDecoration: "none",
  },
  sectionHeading: {
    alignItems: "end",
    borderBottomColor: "var(--line)",
    borderBottomStyle: "solid",
    borderBottomWidth: 1,
    display: "grid",
    gap: 20,
    gridTemplateColumns: {
      default: "72px minmax(0, 1fr) auto",
      "@media (max-width: 700px)": "52px 1fr",
    },
    paddingBottom: 17,
  },
  sectionIndex: {
    fontFamily: display,
    fontSize: "2.8rem",
    fontWeight: 900,
    lineHeight: 0.8,
  },
  kicker: { marginBlock: 0, marginBottom: 5 },
  table: {
    "--kinetic-dense-table-cell-padding": "18px 20px",
    marginTop: 20,
  },
  row: {
    backgroundColor: {
      default: "var(--surface)",
      ":hover": "var(--surface-strong)",
    },
  },
  rowFailure: {
    backgroundColor: "color-mix(in srgb, #b72d25 7%, var(--surface))",
  },
  rowStale: {
    backgroundColor: "color-mix(in srgb, #b57417 8%, var(--surface))",
  },
  draftRow: {
    backgroundColor: "color-mix(in srgb, var(--rey-accent) 6%, var(--surface))",
  },
  identity: {
    alignItems: "start",
    display: "flex",
    gap: 14,
    minWidth: 0,
  },
  identityDetail: { display: "grid", gap: 6, minWidth: 0 },
  ordinal: {
    alignItems: "center",
    backgroundColor: "var(--rey-foreground)",
    color: "var(--rey-background)",
    display: "flex",
    flexShrink: 0,
    fontFamily: mono,
    fontSize: "0.64rem",
    height: 38,
    justifyContent: "center",
    width: 38,
  },
  cellStack: { display: "grid", gap: 7, minWidth: 0 },
  outcomes: {
    display: "grid",
    gap: 5,
    gridTemplateColumns: "repeat(2, minmax(0, 1fr))",
  },
  description: {
    color: "var(--muted)",
    lineHeight: 1.4,
    margin: 0,
  },
  secondary: { color: "var(--muted)", fontSize: "0.72rem" },
  breakable: { overflowWrap: "anywhere" },
  conformance: { display: "grid", gap: 7 },
  conformanceSummary: {
    alignItems: "center",
    display: "flex",
    gap: 12,
    justifyContent: "space-between",
  },
  progressTrack: {
    backgroundColor: "var(--line)",
    display: "block",
    height: 5,
    overflow: "hidden",
  },
  progressFill: {
    backgroundColor: "var(--rey-foreground)",
    display: "block",
    height: "100%",
  },
  progressFailure: { backgroundColor: "#b72d25" },
  progressStale: { backgroundColor: "#b57417" },
  metricTable: { "--kinetic-dense-table-cell-padding": "24px 20px" },
  metricValue: {
    fontFamily: display,
    fontSize: "clamp(1.8rem, 3vw, 3.2rem)",
    fontWeight: 900,
    lineHeight: 1,
  },
  location: {
    borderBottomColor: "currentColor",
    borderBottomStyle: "solid",
    borderBottomWidth: 1,
    color: "inherit",
    display: "inline-block",
    fontFamily: mono,
    fontSize: "0.62rem",
    letterSpacing: "0.08em",
    paddingBottom: 6,
    textDecoration: "none",
  },
});
