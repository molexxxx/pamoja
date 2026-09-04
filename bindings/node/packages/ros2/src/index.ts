/**
 * Ergonomic facade over the generated ROS 2 binding.
 *
 * What makes a topic name legal, what it becomes on the DDS wire, the RIHS type
 * hash that identifies a message definition, and the CDR encoding the payload
 * itself is written in.
 *
 * None of it needs a ROS 2 installation. A gateway written in TypeScript can
 * validate a name, derive the DDS topic and the Zenoh key an `rmw_zenoh` peer
 * subscribes on, and encode a `geometry_msgs/msg/Twist` with no ROS
 * distribution anywhere near it. Driving a live graph does need one, so that
 * stays in the Rust crate.
 *
 * @packageDocumentation
 */

import {
  CdrReader,
  CdrWriter,
  ros2DdsTopic,
  ros2DdsTypeName,
  ros2EntityKey,
  ros2EntityKindPrefix,
  ros2IsFullyQualified,
  ros2IsValidName,
  ros2PercentMangle,
  ros2TwistFromCdr,
  ros2TwistToCdr,
  ros2TypeHashDigest,
  type EntityKindName,
} from '@pamoja/native'

export { CdrReader, CdrWriter } from '@pamoja/native'

export type { Ros2Twist, Ros2Vector3 } from '@pamoja/native'

/**
 * The ROS 2 subsystem a name belongs to, which fixes its DDS prefix.
 *
 * Provided as a runtime object plus a matching string-union type.
 */
export const EntityKind = {
  /** A topic, which takes the `rt` prefix. */
  Topic: 'Topic' as EntityKindName,
  /** The request side of a service, which takes the `rq` prefix. */
  ServiceRequest: 'ServiceRequest' as EntityKindName,
  /** The reply side of a service, which takes the `rr` prefix. */
  ServiceResponse: 'ServiceResponse' as EntityKindName,
} as const

/** One of the {@link EntityKind} choices. */
export type EntityKind = EntityKindName

/** The ROS 2 rules for what a name may be and what it maps onto. */
export const name = {
  /** Reports whether a string is a valid ROS 2 topic or service name. */
  isValid: ros2IsValidName,
  /** Reports whether a name resolves with no namespace applied. */
  isFullyQualified: ros2IsFullyQualified,
  /** Returns the DDS topic prefix a subsystem uses. */
  prefixFor: ros2EntityKindPrefix,
  /**
   * Returns the DDS topic a fully qualified name maps onto, or `null` if the
   * name is not fully qualified.
   */
  ddsTopic: ros2DdsTopic,
  /** Percent-mangles a name the way a DDS partition requires. */
  percentMangle: ros2PercentMangle,
  /**
   * Returns the DDS type name an interface type maps onto, or `null` if the
   * type is not a valid `package/namespace/Type`.
   */
  ddsTypeName: ros2DdsTypeName,
} as const

/** The RIHS type hash that identifies a message definition. */
export const typeHash = {
  /**
   * Returns the 32-byte digest a `RIHS01_` string carries, or `null` if the
   * string is malformed.
   */
  digest: ros2TypeHashDigest,
  /**
   * Builds the Zenoh key an `rmw_zenoh` peer publishes an entity on, or `null`
   * if the name, type, or hash is not usable.
   */
  entityKey: ros2EntityKey,
} as const

/** The CDR encoding a ROS 2 payload travels in. */
export const cdr = {
  /** Encodes a twist into its CDR representation. */
  twistToBytes: ros2TwistToCdr,
  /**
   * Decodes a twist from its CDR representation, or `null` if the bytes are not
   * a well-formed twist.
   */
  twistFromBytes: ros2TwistFromCdr,
  /** Creates an encoder with the encapsulation header already written. */
  writer: () => new CdrWriter(),
  /** Creates a decoder over encoded bytes. */
  reader: (data: Buffer) => new CdrReader(data),
} as const
