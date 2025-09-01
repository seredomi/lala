// import { ToasterProps, toast as sonnerToast } from "sonner";
import { toast as sonnerToast } from "sonner";

import {
  ActionableNotification,
  ActionableNotificationProps,
} from "@carbon/react";
import { FC } from "react";

export type ToastProps = ActionableNotificationProps & {
  id: string | number;
  duration?: number;
  actionCloses?: boolean;
};

export const Toast: FC<ToastProps> = (props: ToastProps) => {
  return (
    <ActionableNotification
      {...props}
      onClose={() => {
        sonnerToast.dismiss(props.id);
      }}
      onActionButtonClick={() => {
        // execute normal specified action
        props.onActionButtonClick && props.onActionButtonClick();
        // also dismiss the toast if actionCloses
        if (props.actionCloses) sonnerToast.dismiss(props.id);
      }}
      lowContrast
      hideCloseButton
    >
      {props.children}
    </ActionableNotification>
  );
};
