import { FileUploaderDropContainer, FormItem } from "@carbon/react";
import { useStore } from "../../../utils/store";

export const FileUploader = () => {
  const { appConfig } = useStore();

  return (
    <div style={{ minWidth: "15rem" }}>
      <FormItem>
        <h4 style={{ fontWeight: 800 }}>upload audio file</h4>
        <p
          style={{ fontStyle: "italic", fontSize: 14 }}
        >{`max file size: ${appConfig?.file_upload?.max_file_size_mb || ""} MB`}</p>
        <p
          style={{ fontStyle: "italic", fontSize: 14, marginBottom: "0.5rem" }}
        >{`supported file types: ${(appConfig?.file_upload?.permitted_file_types || []).join(", ").toUpperCase()}`}</p>
        <FileUploaderDropContainer />
      </FormItem>
    </div>
  );
};
