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
    >
      {props.children}
    </ActionableNotification>
  );
};

export const toast = (props: Omit<ToastProps, "id">) =>
  // @ts-expect-error id comes as a number. unsure why it expects a string
  sonnerToast.custom((id) => <Toast id={id} {...props} />, {
    duration: props.duration,
  });
