//! Fluent builders for constructing test HL7 messages.
//!
//! This module provides builder patterns for creating HL7 v2 messages
//! in tests. The builders support a fluent API for easy message construction.
//!
//! # Example
//!
//! ```rust,ignore
//! use hl7v2_test_utils::builders::MessageBuilder;
//!
//! // Build a simple ADT^A01 message
//! let bytes = MessageBuilder::new()
//!     .with_msh("SendingApp", "SendingFac", "ReceivingApp", "ReceivingFac", "ADT", "A01")
//!     .with_pid("MRN123", "Doe", "John")
//!     .with_pv1("I", "ICU^101")
//!     .build_bytes();
//!
//! // Parse and verify
//! let message = hl7v2_parser::parse(&bytes).unwrap();
//! assert_eq!(message.segments.len(), 3);
//! ```

use hl7v2_model::{Comp, Delims, Field, Message, Rep, Segment};

/// Builder for creating test HL7 messages.
///
/// Provides a fluent API for constructing HL7 v2 messages segment by segment.
///
/// # Example
///
/// ```rust,ignore
/// let message = MessageBuilder::new()
///     .with_msh("App", "Fac", "RecvApp", "RecvFac", "ADT", "A01")
///     .with_pid("MRN123", "Doe", "John")
///     .build();
/// ```
#[derive(Debug, Clone)]
pub struct MessageBuilder {
    segments: Vec<Segment>,
    delims: Delims,
    message_control_id: Option<String>,
    timestamp: Option<String>,
}

impl Default for MessageBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl MessageBuilder {
    /// Create a new message builder with no segments.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let builder = MessageBuilder::new();
    /// ```
    pub fn new() -> Self {
        Self {
            segments: Vec::new(),
            delims: Delims::default(),
            message_control_id: None,
            timestamp: None,
        }
    }

    /// Create a builder pre-configured for an ADT^A01 message.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let builder = MessageBuilder::adt_a01();
    /// ```
    pub fn adt_a01() -> Self {
        Self::new()
            .with_msh("SendingApp", "SendingFac", "ReceivingApp", "ReceivingFac", "ADT", "A01")
    }

    /// Create a builder pre-configured for an ADT^A04 message.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let builder = MessageBuilder::adt_a04();
    /// ```
    pub fn adt_a04() -> Self {
        Self::new()
            .with_msh("RegSys", "Hospital", "ADT", "Hospital", "ADT", "A04")
    }

    /// Create a builder pre-configured for an ORU^R01 message.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let builder = MessageBuilder::oru_r01();
    /// ```
    pub fn oru_r01() -> Self {
        Self::new()
            .with_msh("LabSys", "Lab", "LIS", "Hospital", "ORU", "R01")
    }

    /// Set custom delimiters for the message.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let delims = Delims { field: '#', comp: '$', rep: '*', esc: '@', sub: '!' };
    /// let builder = MessageBuilder::new().with_delims(delims);
    /// ```
    pub fn with_delims(mut self, delims: Delims) -> Self {
        self.delims = delims;
        self
    }

    /// Set a custom message control ID (MSH-10).
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let builder = MessageBuilder::new()
    ///     .with_msh("App", "Fac", "Recv", "RecvFac", "ADT", "A01")
    ///     .with_message_control_id("MSG12345");
    /// ```
    pub fn with_message_control_id(mut self, id: impl Into<String>) -> Self {
        self.message_control_id = Some(id.into());
        self
    }

    /// Set a custom timestamp for the message (MSH-7).
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let builder = MessageBuilder::new()
    ///     .with_msh("App", "Fac", "Recv", "RecvFac", "ADT", "A01")
    ///     .with_timestamp("20250128152312");
    /// ```
    pub fn with_timestamp(mut self, timestamp: impl Into<String>) -> Self {
        self.timestamp = Some(timestamp.into());
        self
    }

    /// Add an MSH (Message Header) segment.
    ///
    /// This is typically the first segment added to a message.
    ///
    /// # Arguments
    ///
    /// * `sending_app` - Sending application (MSH-3)
    /// * `sending_fac` - Sending facility (MSH-4)
    /// * `receiving_app` - Receiving application (MSH-5)
    /// * `receiving_fac` - Receiving facility (MSH-6)
    /// * `message_type` - Message type code (MSH-9-1)
    /// * `trigger_event` - Trigger event code (MSH-9-2)
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let builder = MessageBuilder::new()
    ///     .with_msh("MyApp", "MyFac", "RecvApp", "RecvFac", "ADT", "A01");
    /// ```
    pub fn with_msh(
        mut self,
        sending_app: &str,
        sending_fac: &str,
        receiving_app: &str,
        receiving_fac: &str,
        message_type: &str,
        trigger_event: &str,
    ) -> Self {
        let timestamp = self.timestamp.clone().unwrap_or_else(|| "20250128152312".to_string());
        let control_id = self.message_control_id.clone().unwrap_or_else(|| {
            format!("MSG{}", chrono_timestamp())
        });

        let mut segment = Segment::new(b"MSH");
        
        // MSH-1 is the field separator (implicit in the delims)
        // MSH-2 is the encoding characters (implicit in the delims)
        
        // MSH-3: Sending Application
        segment.add_field(Field::from_text(sending_app));
        // MSH-4: Sending Facility
        segment.add_field(Field::from_text(sending_fac));
        // MSH-5: Receiving Application
        segment.add_field(Field::from_text(receiving_app));
        // MSH-6: Receiving Facility
        segment.add_field(Field::from_text(receiving_fac));
        // MSH-7: Date/Time of Message
        segment.add_field(Field::from_text(&timestamp));
        // MSH-8: Security (empty)
        segment.add_field(Field::new());
        // MSH-9: Message Type (componentized)
        segment.add_field(make_component_field(&[message_type, trigger_event]));
        // MSH-10: Message Control ID
        segment.add_field(Field::from_text(&control_id));
        // MSH-11: Processing ID
        segment.add_field(Field::from_text("P"));
        // MSH-12: Version ID
        segment.add_field(Field::from_text("2.5.1"));

        self.segments.push(segment);
        self
    }

    /// Add a PID (Patient Identification) segment.
    ///
    /// # Arguments
    ///
    /// * `mrn` - Medical Record Number (PID-3)
    /// * `last_name` - Patient's last name (PID-5-1)
    /// * `first_name` - Patient's first name (PID-5-2)
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let builder = MessageBuilder::new()
    ///     .with_pid("MRN12345", "Doe", "John");
    /// ```
    pub fn with_pid(self, mrn: &str, last_name: &str, first_name: &str) -> Self {
        self.with_pid_full(mrn, last_name, first_name, None, None, None)
    }

    /// Add a PID segment with full patient information.
    ///
    /// # Arguments
    ///
    /// * `mrn` - Medical Record Number
    /// * `last_name` - Patient's last name
    /// * `first_name` - Patient's first name
    /// * `middle_name` - Optional middle name
    /// * `dob` - Optional date of birth (YYYYMMDD format)
    /// * `sex` - Optional sex (M/F/U/O/A/N)
    pub fn with_pid_full(
        mut self,
        mrn: &str,
        last_name: &str,
        first_name: &str,
        middle_name: Option<&str>,
        dob: Option<&str>,
        sex: Option<&str>,
    ) -> Self {
        let mut segment = Segment::new(b"PID");
        
        // PID-1: Set ID
        segment.add_field(Field::from_text("1"));
        // PID-2: Patient ID (External) - often empty
        segment.add_field(Field::new());
        // PID-3: Patient Identifier List (MRN)
        segment.add_field(make_component_field(&[mrn, "", "", "HOSP", "MR"]));
        // PID-4: Alternate Patient ID - often empty
        segment.add_field(Field::new());
        // PID-5: Patient Name
        let name_components = match middle_name {
            Some(middle) => vec![last_name, first_name, middle],
            None => vec![last_name, first_name],
        };
        segment.add_field(make_component_field(&name_components));
        // PID-6: Mother's Maiden Name - often empty
        segment.add_field(Field::new());
        // PID-7: Date/Time of Birth
        segment.add_field(Field::from_text(dob.unwrap_or("19800101")));
        // PID-8: Administrative Sex
        segment.add_field(Field::from_text(sex.unwrap_or("M")));
        // PID-9: Patient Alias - often empty
        segment.add_field(Field::new());
        // PID-10: Race - often empty
        segment.add_field(Field::new());
        // PID-11: Patient Address - often empty
        segment.add_field(Field::new());
        // PID-12: County Code - often empty
        segment.add_field(Field::new());
        // PID-13: Phone Number - often empty
        segment.add_field(Field::new());
        // PID-14: Phone Number (Business) - often empty
        segment.add_field(Field::new());
        // PID-15: Primary Language - often empty
        segment.add_field(Field::new());
        // PID-16: Marital Status - often empty
        segment.add_field(Field::new());
        // PID-17: Religion - often empty
        segment.add_field(Field::new());
        // PID-18: Patient Account Number - often empty
        segment.add_field(Field::new());
        // PID-19: SSN - often empty
        segment.add_field(Field::new());

        self.segments.push(segment);
        self
    }

    /// Add a PV1 (Patient Visit) segment.
    ///
    /// # Arguments
    ///
    /// * `patient_class` - Patient class code (e.g., "I" for inpatient)
    /// * `location` - Patient location (e.g., "ICU^101^01")
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let builder = MessageBuilder::new()
    ///     .with_pv1("I", "ICU^101^01");
    /// ```
    pub fn with_pv1(mut self, patient_class: &str, location: &str) -> Self {
        let mut segment = Segment::new(b"PV1");
        
        // PV1-1: Set ID
        segment.add_field(Field::from_text("1"));
        // PV1-2: Patient Class
        segment.add_field(Field::from_text(patient_class));
        // PV1-3: Assigned Patient Location
        let location_parts: Vec<&str> = location.split('^').collect();
        segment.add_field(make_component_field(&location_parts));
        // PV1-4 through PV1-51: Various optional fields
        for _ in 4..=51 {
            segment.add_field(Field::new());
        }

        self.segments.push(segment);
        self
    }

    /// Add an OBX (Observation Result) segment.
    ///
    /// # Arguments
    ///
    /// * `set_id` - Set ID (OBX-1)
    /// * `value_type` - Value type code (OBX-2)
    /// * `observation_id` - Observation identifier (OBX-3)
    /// * `value` - Observation value (OBX-5)
    /// * `units` - Units (OBX-6)
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let builder = MessageBuilder::new()
    ///     .with_obx("1", "NM", "WBC^White Blood Count", "7.5", "10^9/L");
    /// ```
    pub fn with_obx(
        mut self,
        set_id: &str,
        value_type: &str,
        observation_id: &str,
        value: &str,
        units: &str,
    ) -> Self {
        let mut segment = Segment::new(b"OBX");
        
        // OBX-1: Set ID
        segment.add_field(Field::from_text(set_id));
        // OBX-2: Value Type
        segment.add_field(Field::from_text(value_type));
        // OBX-3: Observation Identifier
        let obs_parts: Vec<&str> = observation_id.split('^').collect();
        segment.add_field(make_component_field(&obs_parts));
        // OBX-4: Observation Sub-ID (empty)
        segment.add_field(Field::new());
        // OBX-5: Observation Value
        segment.add_field(Field::from_text(value));
        // OBX-6: Units
        let unit_parts: Vec<&str> = units.split('^').collect();
        segment.add_field(make_component_field(&unit_parts));
        // OBX-7: Reference Range (empty)
        segment.add_field(Field::new());
        // OBX-8: Abnormal Flags (empty)
        segment.add_field(Field::new());
        // OBX-9: Probability (empty)
        segment.add_field(Field::new());
        // OBX-10: Nature of Abnormal Test (empty)
        segment.add_field(Field::new());
        // OBX-11: Observation Result Status
        segment.add_field(Field::from_text("F"));

        self.segments.push(segment);
        self
    }

    /// Add an OBR (Observation Request) segment.
    ///
    /// # Arguments
    ///
    /// * `set_id` - Set ID (OBR-1)
    /// * `placer_order` - Placer Order Number (OBR-2)
    /// * `filler_order` - Filler Order Number (OBR-3)
    /// * `universal_service` - Universal Service Identifier (OBR-4)
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let builder = MessageBuilder::new()
    ///     .with_obr("1", "ORD123", "FIL456", "CBC^Complete Blood Count");
    /// ```
    pub fn with_obr(
        mut self,
        set_id: &str,
        placer_order: &str,
        filler_order: &str,
        universal_service: &str,
    ) -> Self {
        let mut segment = Segment::new(b"OBR");
        
        // OBR-1: Set ID
        segment.add_field(Field::from_text(set_id));
        // OBR-2: Placer Order Number
        segment.add_field(Field::from_text(placer_order));
        // OBR-3: Filler Order Number
        segment.add_field(Field::from_text(filler_order));
        // OBR-4: Universal Service Identifier
        let service_parts: Vec<&str> = universal_service.split('^').collect();
        segment.add_field(make_component_field(&service_parts));
        // OBR-5 through OBR-47: Various optional fields
        for _ in 5..=47 {
            segment.add_field(Field::new());
        }

        self.segments.push(segment);
        self
    }

    /// Add an EVN (Event Type) segment.
    ///
    /// # Arguments
    ///
    /// * `event_type` - Event type code (EVN-1)
    /// * `datetime` - Event date/time (EVN-2)
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let builder = MessageBuilder::new()
    ///     .with_evn("A01", "20250128152312");
    /// ```
    pub fn with_evn(mut self, event_type: &str, datetime: &str) -> Self {
        let mut segment = Segment::new(b"EVN");
        
        // EVN-1: Event Type Code
        segment.add_field(Field::from_text(event_type));
        // EVN-2: Recorded Date/Time
        segment.add_field(Field::from_text(datetime));
        // EVN-3 through EVN-7: Optional fields
        for _ in 3..=7 {
            segment.add_field(Field::new());
        }

        self.segments.push(segment);
        self
    }

    /// Add an NK1 (Next of Kin) segment.
    ///
    /// # Arguments
    ///
    /// * `set_id` - Set ID (NK1-1)
    /// * `name` - Next of kin name (NK1-2)
    /// * `relationship` - Relationship code (NK1-3)
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let builder = MessageBuilder::new()
    ///     .with_nk1("1", "Doe^Jane", "SPO");
    /// ```
    pub fn with_nk1(mut self, set_id: &str, name: &str, relationship: &str) -> Self {
        let mut segment = Segment::new(b"NK1");
        
        // NK1-1: Set ID
        segment.add_field(Field::from_text(set_id));
        // NK1-2: Name
        let name_parts: Vec<&str> = name.split('^').collect();
        segment.add_field(make_component_field(&name_parts));
        // NK1-3: Relationship
        segment.add_field(Field::from_text(relationship));

        self.segments.push(segment);
        self
    }

    /// Add an AL1 (Patient Allergy) segment.
    ///
    /// # Arguments
    ///
    /// * `set_id` - Set ID (AL1-1)
    /// * `allergy_type` - Allergy type code (AL1-2)
    /// * `allergen` - Allergen code and description (AL1-3)
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let builder = MessageBuilder::new()
    ///     .with_al1("1", "DA", "Penicillin");
    /// ```
    pub fn with_al1(mut self, set_id: &str, allergy_type: &str, allergen: &str) -> Self {
        let mut segment = Segment::new(b"AL1");
        
        // AL1-1: Set ID
        segment.add_field(Field::from_text(set_id));
        // AL1-2: Allergy Type
        segment.add_field(Field::from_text(allergy_type));
        // AL1-3: Allergen
        segment.add_field(Field::from_text(allergen));

        self.segments.push(segment);
        self
    }

    /// Add a DG1 (Diagnosis) segment.
    ///
    /// # Arguments
    ///
    /// * `set_id` - Set ID (DG1-1)
    /// * `coding_method` - Coding method (DG1-2)
    /// * `diagnosis_code` - Diagnosis code and description (DG1-3)
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let builder = MessageBuilder::new()
    ///     .with_dg1("1", "I10", "J18.9^Pneumonia");
    /// ```
    pub fn with_dg1(mut self, set_id: &str, coding_method: &str, diagnosis_code: &str) -> Self {
        let mut segment = Segment::new(b"DG1");
        
        // DG1-1: Set ID
        segment.add_field(Field::from_text(set_id));
        // DG1-2: Diagnosis Coding Method
        segment.add_field(Field::from_text(coding_method));
        // DG1-3: Diagnosis Code
        let code_parts: Vec<&str> = diagnosis_code.split('^').collect();
        segment.add_field(make_component_field(&code_parts));

        self.segments.push(segment);
        self
    }

    /// Add a custom segment.
    ///
    /// # Arguments
    ///
    /// * `segment` - The segment to add
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let mut custom_segment = Segment::new(b"ZPV");
    /// custom_segment.add_field(Field::from_text("custom value"));
    /// let builder = MessageBuilder::new().with_segment(custom_segment);
    /// ```
    pub fn with_segment(mut self, segment: Segment) -> Self {
        self.segments.push(segment);
        self
    }

    /// Add a raw segment from a string.
    ///
    /// # Arguments
    ///
    /// * `segment_str` - The segment as a string (without trailing \r)
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let builder = MessageBuilder::new()
    ///     .with_raw_segment("ZPV|1|Custom|Value");
    /// ```
    pub fn with_raw_segment(mut self, segment_str: &str) -> Self {
        if let Ok(segment) = parse_raw_segment(segment_str, &self.delims) {
            self.segments.push(segment);
        }
        self
    }

    /// Build the message and return a Message struct.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let message = MessageBuilder::new()
    ///     .with_msh("App", "Fac", "Recv", "RecvFac", "ADT", "A01")
    ///     .build();
    /// ```
    pub fn build(self) -> Message {
        Message {
            delims: self.delims,
            segments: self.segments,
            charsets: Vec::new(),
        }
    }

    /// Build the message and return it as HL7-formatted bytes.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let bytes = MessageBuilder::new()
    ///     .with_msh("App", "Fac", "Recv", "RecvFac", "ADT", "A01")
    ///     .build_bytes();
    /// ```
    pub fn build_bytes(self) -> Vec<u8> {
        let message = self.build();
        write_message(&message).into_bytes()
    }

    /// Build the message and return it as an HL7-formatted string.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let hl7_string = MessageBuilder::new()
    ///     .with_msh("App", "Fac", "Recv", "RecvFac", "ADT", "A01")
    ///     .build_string();
    /// ```
    pub fn build_string(self) -> String {
        let message = self.build();
        write_message(&message)
    }
}

/// Builder for creating individual HL7 segments.
///
/// Provides a fluent API for constructing segments field by field.
///
/// # Example
///
/// ```rust,ignore
/// let segment = SegmentBuilder::new("PID")
///     .field("1")
///     .field("MRN123")
///     .component_field(&["Doe", "John"])
///     .build();
/// ```
#[derive(Debug, Clone)]
pub struct SegmentBuilder {
    id: [u8; 3],
    fields: Vec<Field>,
    delims: Delims,
}

impl SegmentBuilder {
    /// Create a new segment builder with the given segment ID.
    ///
    /// # Arguments
    ///
    /// * `id` - The 3-character segment ID (e.g., "PID", "PV1")
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let builder = SegmentBuilder::new("PID");
    /// ```
    pub fn new(id: &str) -> Self {
        let id_bytes = if id.len() >= 3 {
            [id.as_bytes()[0], id.as_bytes()[1], id.as_bytes()[2]]
        } else {
            [b'?', b'?', b'?']
        };
        
        Self {
            id: id_bytes,
            fields: Vec::new(),
            delims: Delims::default(),
        }
    }

    /// Set custom delimiters for the segment.
    pub fn with_delims(mut self, delims: Delims) -> Self {
        self.delims = delims;
        self
    }

    /// Add a simple text field.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let segment = SegmentBuilder::new("PID")
    ///     .field("1")
    ///     .field("MRN123")
    ///     .build();
    /// ```
    pub fn field(mut self, value: &str) -> Self {
        self.fields.push(Field::from_text(value));
        self
    }

    /// Add an empty field.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let segment = SegmentBuilder::new("PID")
    ///     .field("1")
    ///     .empty_field()  // PID-2
    ///     .field("MRN123")
    ///     .build();
    /// ```
    pub fn empty_field(mut self) -> Self {
        self.fields.push(Field::new());
        self
    }

    /// Add a field with multiple components.
    ///
    /// # Arguments
    ///
    /// * `components` - Slice of component values
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let segment = SegmentBuilder::new("PID")
    ///     .field("1")
    ///     .component_field(&["Doe", "John", "A"])
    ///     .build();
    /// ```
    pub fn component_field(mut self, components: &[&str]) -> Self {
        self.fields.push(make_component_field(components));
        self
    }

    /// Add a field with multiple repetitions.
    ///
    /// # Arguments
    ///
    /// * `repetitions` - Slice of slices, where each inner slice is a repetition's components
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let segment = SegmentBuilder::new("PID")
    ///     .field("1")
    ///     .repetition_field(&[
    ///         &["Doe", "John"],
    ///         &["Smith", "Jane"],
    ///     ])
    ///     .build();
    /// ```
    pub fn repetition_field(mut self, repetitions: &[&[&str]]) -> Self {
        let mut field = Field::new();
        for rep_components in repetitions {
            let mut rep = Rep::new();
            let mut comp = Comp::new();
            for value in *rep_components {
                comp.subs.push(hl7v2_model::Atom::text(*value));
            }
            rep.comps.push(comp);
            field.reps.push(rep);
        }
        self.fields.push(field);
        self
    }

    /// Add a raw field (parsed from string).
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let segment = SegmentBuilder::new("PID")
    ///     .field("1")
    ///     .raw_field("123456^^^HOSP^MR")
    ///     .build();
    /// ```
    pub fn raw_field(mut self, field_str: &str) -> Self {
        self.fields.push(parse_raw_field(field_str, &self.delims));
        self
    }

    /// Build the segment.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let segment = SegmentBuilder::new("PID")
    ///     .field("1")
    ///     .field("MRN123")
    ///     .build();
    /// ```
    pub fn build(self) -> Segment {
        Segment {
            id: self.id,
            fields: self.fields,
        }
    }

    /// Build the segment and return it as an HL7-formatted string.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let segment_str = SegmentBuilder::new("PID")
    ///     .field("1")
    ///     .field("MRN123")
    ///     .build_string();
    /// ```
    pub fn build_string(self) -> String {
        let segment = self.build();
        write_segment(&segment, &Delims::default())
    }
}

// Helper functions

/// Generate a simple timestamp for message control IDs.
fn chrono_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}", duration.as_millis() % 1_000_000)
}

/// Create a field with multiple components from a slice of values.
fn make_component_field(components: &[&str]) -> Field {
    let mut comp = Comp::new();
    for value in components {
        comp.subs.push(hl7v2_model::Atom::text(*value));
    }
    let mut rep = Rep::new();
    rep.comps.push(comp);
    Field { reps: vec![rep] }
}

/// Write a message to an HL7-formatted string.
fn write_message(message: &Message) -> String {
    let mut result = String::new();
    for segment in &message.segments {
        result.push_str(&write_segment(segment, &message.delims));
        result.push('\r');
    }
    result
}

/// Write a segment to an HL7-formatted string.
fn write_segment(segment: &Segment, delims: &Delims) -> String {
    let mut result = segment.id_str().to_string();
    
    // Special handling for MSH segment
    // MSH-1 is the field separator (implicit after MSH)
    // MSH-2 is the encoding characters
    if segment.id_str() == "MSH" {
        // Write MSH-1 and MSH-2 (field separator + encoding characters)
        result.push(delims.field);
        result.push(delims.comp);
        result.push(delims.rep);
        result.push(delims.esc);
        result.push(delims.sub);
        
        // Write remaining fields starting from MSH-3
        for field in &segment.fields {
            result.push(delims.field);
            result.push_str(&write_field(field, delims));
        }
    } else {
        for field in &segment.fields {
            result.push(delims.field);
            result.push_str(&write_field(field, delims));
        }
    }
    
    result
}

/// Write a field to an HL7-formatted string.
fn write_field(field: &Field, delims: &Delims) -> String {
    let reps: Vec<String> = field.reps.iter()
        .map(|rep| write_rep(rep, delims))
        .collect();
    reps.join(&delims.rep.to_string())
}

/// Write a repetition to an HL7-formatted string.
fn write_rep(rep: &Rep, delims: &Delims) -> String {
    let comps: Vec<String> = rep.comps.iter()
        .map(|comp| write_comp(comp, delims))
        .collect();
    comps.join(&delims.comp.to_string())
}

/// Write a component to an HL7-formatted string.
fn write_comp(comp: &Comp, delims: &Delims) -> String {
    let subs: Vec<String> = comp.subs.iter()
        .map(write_atom)
        .collect();
    subs.join(&delims.sub.to_string())
}

/// Write a subcomponent to an HL7-formatted string.
fn write_atom(atom: &hl7v2_model::Atom) -> String {
    match atom {
        hl7v2_model::Atom::Text(t) => t.clone(),
        hl7v2_model::Atom::Null => String::new(),
    }
}

/// Parse a raw segment string into a Segment struct.
fn parse_raw_segment(segment_str: &str, delims: &Delims) -> Result<Segment, String> {
    if segment_str.len() < 3 {
        return Err("Segment too short".to_string());
    }
    
    let id: [u8; 3] = [
        segment_str.as_bytes()[0],
        segment_str.as_bytes()[1],
        segment_str.as_bytes()[2],
    ];
    
    let mut segment = Segment::new(&id);
    
    if segment_str.len() > 3 {
        let field_sep = segment_str.chars().nth(3).unwrap_or(delims.field);
        let fields_str = &segment_str[4..];
        
        for field_str in fields_str.split(field_sep) {
            segment.add_field(parse_raw_field(field_str, delims));
        }
    }
    
    Ok(segment)
}

/// Parse a raw field string into a Field struct.
fn parse_raw_field(field_str: &str, delims: &Delims) -> Field {
    let mut field = Field::new();
    
    for rep_str in field_str.split(delims.rep) {
        let mut rep = Rep::new();
        for comp_str in rep_str.split(delims.comp) {
            let mut comp = Comp::new();
            for sub_str in comp_str.split(delims.sub) {
                if sub_str.is_empty() {
                    comp.subs.push(hl7v2_model::Atom::Null);
                } else {
                    comp.subs.push(hl7v2_model::Atom::text(sub_str));
                }
            }
            rep.comps.push(comp);
        }
        field.reps.push(rep);
    }
    
    field
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_builder_new() {
        let builder = MessageBuilder::new();
        assert!(builder.segments.is_empty());
    }

    #[test]
    fn test_message_builder_adt_a01() {
        let bytes = MessageBuilder::adt_a01()
            .with_pid("MRN123", "Doe", "John")
            .build_bytes();
        
        let message = hl7v2_parser::parse(&bytes).unwrap();
        assert_eq!(message.segments.len(), 2);
    }

    #[test]
    fn test_message_builder_full_message() {
        let bytes = MessageBuilder::new()
            .with_msh("App", "Fac", "Recv", "RecvFac", "ADT", "A01")
            .with_pid("MRN123", "Doe", "John")
            .with_pv1("I", "ICU^101")
            .build_bytes();
        
        let message = hl7v2_parser::parse(&bytes).unwrap();
        assert_eq!(message.segments.len(), 3);
    }

    #[test]
    fn test_message_builder_with_obx() {
        let bytes = MessageBuilder::oru_r01()
            .with_pid("MRN789", "Patient", "Test")
            .with_obr("1", "ORD123", "FIL456", "CBC^Complete Blood Count")
            .with_obx("1", "NM", "WBC^White Blood Count", "7.5", "10^9/L")
            .build_bytes();
        
        let message = hl7v2_parser::parse(&bytes).unwrap();
        assert_eq!(message.segments.len(), 4);
    }

    #[test]
    fn test_message_builder_custom_control_id() {
        let bytes = MessageBuilder::new()
            .with_msh("App", "Fac", "Recv", "RecvFac", "ADT", "A01")
            .with_message_control_id("CUSTOM123")
            .build_bytes();
        
        let message = hl7v2_parser::parse(&bytes).unwrap();
        // Verify the message was created successfully
        assert_eq!(message.segments.len(), 1);
    }

    #[test]
    fn test_segment_builder() {
        let segment = SegmentBuilder::new("PID")
            .field("1")
            .empty_field()
            .field("MRN123")
            .component_field(&["Doe", "John"])
            .build();
        
        assert_eq!(segment.id_str(), "PID");
        assert_eq!(segment.fields.len(), 4);
    }

    #[test]
    fn test_segment_builder_string() {
        let segment_str = SegmentBuilder::new("PID")
            .field("1")
            .field("MRN123")
            .build_string();
        
        assert!(segment_str.starts_with("PID|"));
        assert!(segment_str.contains("MRN123"));
    }

    #[test]
    fn test_message_builder_with_raw_segment() {
        let bytes = MessageBuilder::new()
            .with_msh("App", "Fac", "Recv", "RecvFac", "ADT", "A01")
            .with_raw_segment("ZPV|1|Custom|Value")
            .build_bytes();
        
        let message = hl7v2_parser::parse(&bytes).unwrap();
        assert_eq!(message.segments.len(), 2);
        assert_eq!(message.segments[1].id_str(), "ZPV");
    }
}
