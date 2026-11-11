const base = {
  width: 16,
  height: 16,
  viewBox: "0 0 16 16",
  fill: "none",
  stroke: "currentColor",
  strokeWidth: 1.5,
  strokeLinecap: "round" as const,
  strokeLinejoin: "round" as const,
  "aria-hidden": true
};

export const Icon = {
  Search: (
    <svg {...base}>
      <circle cx="7" cy="7" r="4.2" />
      <path d="m10.2 10.2 3 3" />
    </svg>
  ),
  Plus: (
    <svg {...base}>
      <path d="M8 3.5v9M3.5 8h9" />
    </svg>
  ),
  Check: (
    <svg {...base}>
      <path d="m3.5 8.4 3 3 6-6.8" />
    </svg>
  ),
  Close: (
    <svg {...base}>
      <path d="m4 4 8 8M12 4l-8 8" />
    </svg>
  ),
  ChevronDown: (
    <svg {...base} width={12} height={12} viewBox="0 0 12 12">
      <path d="m3 4.6 3 3 3-3" />
    </svg>
  ),
  Folder: (
    <svg {...base}>
      <path d="M2 4.5c0-.6.4-1 1-1h3l1.2 1.4H13c.6 0 1 .4 1 1V12c0 .6-.4 1-1 1H3c-.6 0-1-.4-1-1V4.5Z" />
    </svg>
  ),
  Sun: (
    <svg {...base}>
      <circle cx="8" cy="8" r="3" />
      <path d="M8 1.8v1.4M8 12.8v1.4M1.8 8h1.4M12.8 8h1.4M3.6 3.6l1 1M11.4 11.4l1 1M12.4 3.6l-1 1M4.6 11.4l-1 1" />
    </svg>
  ),
  Moon: (
    <svg {...base}>
      <path d="M13.2 10.2A5.4 5.4 0 0 1 5.8 2.8 5.4 5.4 0 1 0 13.2 10.2Z" />
    </svg>
  ),
  Server: (
    <svg {...base}>
      <rect x="2.2" y="2.6" width="11.6" height="4.4" rx="1.4" />
      <rect x="2.2" y="9" width="11.6" height="4.4" rx="1.4" />
      <path d="M5 4.8h.01M5 11.2h.01" strokeWidth="2" />
    </svg>
  ),
  Terminal: (
    <svg {...base}>
      <rect x="2" y="3" width="12" height="10" rx="2" />
      <path d="m5 6.5 2 1.7-2 1.7M8.6 10h2.6" />
    </svg>
  ),
  Trash: (
    <svg {...base}>
      <path d="M2.8 4.6h10.4M6.2 4.6V3.2c0-.4.3-.7.7-.7h2.2c.4 0 .7.3.7.7v1.4M4.2 4.6l.6 8c0 .4.4.8.8.8h4.8c.4 0 .8-.4.8-.8l.6-8" />
    </svg>
  ),
  Copy: (
    <svg {...base}>
      <rect x="5.6" y="5.6" width="8" height="8" rx="1.6" />
      <path d="M10.4 5.6V3.9c0-.8-.6-1.5-1.4-1.5H3.9c-.8 0-1.4.7-1.4 1.5v5.2c0 .8.6 1.4 1.4 1.4h1.7" />
    </svg>
  ),
  Refresh: (
    <svg {...base}>
      <path d="M13 7.4A5 5 0 1 1 11 3.9" />
      <path d="M13.2 2.4v3.1h-3.1" />
    </svg>
  ),
  External: (
    <svg {...base}>
      <path d="M9 2.8h4.2V7M13.2 2.8 7.6 8.4" />
      <path d="M11.6 9.6v2.6c0 .6-.4 1-1 1H3.8c-.6 0-1-.4-1-1V5.4c0-.6.4-1 1-1h2.6" />
    </svg>
  ),
  Spinner: <span className="spinner" aria-hidden="true" />,
  Settings: (
    <svg {...base}>
      <path d="M2.5 4.5h11M2.5 11.5h11" />
      <circle cx="6" cy="4.5" r="1.6" />
      <circle cx="10.5" cy="11.5" r="1.6" />
    </svg>
  ),
  Edit: (
    <svg {...base}>
      <path d="M10.6 2.6a1.6 1.6 0 0 1 2.3 2.3L5.4 12.4l-3 .7.7-3 7.5-7.5Z" />
    </svg>
  ),
  Eye: (
    <svg {...base}>
      <path d="M1.6 8S3.8 4 8 4s6.4 4 6.4 4S12.2 12 8 12 1.6 8 1.6 8Z" />
      <circle cx="8" cy="8" r="1.8" />
    </svg>
  ),
  File: (
    <svg {...base} width={12} height={12} viewBox="0 0 16 16">
      <path d="M4 2.4h4.6L12 5.8v7.8H4V2.4Z" />
      <path d="M8.4 2.6v3.2h3.2" />
    </svg>
  ),
  Import: (
    <svg {...base}>
      <path d="M8 2.4v7.2M5.2 6.8 8 9.6l2.8-2.8" />
      <path d="M3 11.2v1.2c0 .6.4 1 1 1h8c.6 0 1-.4 1-1v-1.2" />
    </svg>
  )
};
