/** The dcmjs 0.52.2 surface this package calls. The package ships no types. */
declare module "dcmjs" {
  interface DicomDict {
    meta: unknown;
    dict: unknown;
    write(): ArrayBuffer | Uint8Array;
  }

  interface ReportWriter {
    contentItem(
      source: { StudyInstanceUID: string; SeriesInstanceUID: string },
      options?: { PersonName: string },
    ): Record<string, unknown>;
  }

  interface DcmjsApi {
    data: {
      DicomDict: new (meta: unknown) => DicomDict;
      DicomMessage: {
        readFile(buffer: ArrayBuffer, options?: { ignoreErrors?: boolean }): DicomDict;
      };
      DicomMetaDictionary: {
        uid(): string;
        naturalizeDataset(dataset: unknown): Record<string, unknown>;
        denaturalizeDataset(dataset: unknown): unknown;
      };
    };
    utilities: {
      TID1500: {
        TID1500MeasurementReport: new (groups: {
          TID1501MeasurementGroups: unknown[];
        }) => ReportWriter;
        TID1501MeasurementGroup: new (measurements: unknown[]) => unknown;
      };
    };
  }

  const dcmjs: DcmjsApi;
  export default dcmjs;
}
