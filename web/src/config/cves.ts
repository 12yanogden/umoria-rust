export type CveRecord = {
  id: `CVE-${number}-${number}`;
  title: string;
  date: string;
};

/** CVE demo surface removed for Umoria. Kept empty so leftover module imports still typecheck. */
export const cveRecords: CveRecord[] = [];
