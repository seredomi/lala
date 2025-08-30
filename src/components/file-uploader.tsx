import { FileUploaderDropContainer, FormItem } from "@carbon/react";
import { useRecoilValue } from "recoil";
import { appConfigState } from "../utils/state";

export const FileUploader = () => {
  const appConfig = useRecoilValue(appConfigState);

  return (
    <FormItem>
      <h4 style={{ fontWeight: 800 }}>upload audio file</h4>
      <p
        style={{ fontStyle: "italic" }}
      >{`max file size: ${appConfig?.file_upload.max_file_size_mb || ""} MB`}</p>
      <p
        style={{ fontStyle: "italic", marginBottom: "0.5rem" }}
      >{`supported file types: ${(appConfig?.file_upload.permitted_file_types || []).join(", ").toUpperCase()}`}</p>
      <FileUploaderDropContainer />
    </FormItem>
  );
};
