import { Link, Stack } from "@carbon/react";
import { Text } from "@carbon/react/lib/components/Text";
import { openUrl } from "@tauri-apps/plugin-opener";

export const AboutView = () => {
  return (
    <>
      <h4 style={{ fontWeight: 800 }}>about</h4>
      <Stack orientation="horizontal">
        <div style={{ marginTop: "1rem" }}>
          <Text>created by&nbsp;</Text>
          <Link onClick={() => openUrl("https://github.com/seredomi")}>
            sereno
          </Link>
        </div>
      </Stack>
    </>
  );
};
